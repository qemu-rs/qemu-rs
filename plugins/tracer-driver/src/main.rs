use anyhow::{Error, Result, anyhow};
use clap::Parser;
use rand::{Rng, distr::Alphanumeric, rng};
use serde_cbor::Deserializer;
use serde_json::to_string;
use std::process::{Command, Stdio};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write, stdout},
    os::unix::net::UnixListener,
    path::{Path, PathBuf},
};
use tokio::{
    fs::{read, remove_file},
    join, main, spawn,
    task::spawn_blocking,
};
use tracer_events::Event;
use tracing::{debug, level_filters::LevelFilter, subscriber::set_global_default};
use tracing_subscriber::fmt;

fn tmp(prefix: &str, suffix: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}{}{}",
        prefix,
        rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect::<String>(),
        suffix
    ))
}

#[derive(Parser, Debug, Clone)]
/// Run QEMU with a plugin that logs events. To pass arguments to QEMU, use the QEMU environment
/// variables.
struct Args {
    #[clap(short = 'Q', long, default_value = "qemu-x86_64")]
    /// The alternative QEMU binary to use
    pub qemu_bin: PathBuf,
    #[clap(short = 'P', long)]
    /// The path to the plugin to use
    pub plugin_path: PathBuf,
    #[clap(short = 'i', long)]
    /// Whether instructions should be logged
    pub log_insns: bool,
    #[clap(short = 'm', long)]
    /// Whether memory accesses should be logged
    pub log_mem: bool,
    #[clap(short = 's', long)]
    /// Whether syscalls should be logged
    pub log_syscalls: bool,
    #[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
    #[clap(short = 'r', long)]
    /// Whether registers should be logged
    pub log_registers: bool,
    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3",
        feature = "plugin-api-v4",
        feature = "plugin-api-v5",
    )))]
    #[clap(short = 'd', long)]
    /// Whether discontinuities should be logged
    pub log_discons: bool,
    #[clap(short = 'a', long)]
    /// Whether all events should be logged
    pub log_all: bool,
    #[clap(short = 'I', long)]
    /// An input file to use as the program's stdin, otherwise the driver's stdin is used
    pub input_file: Option<PathBuf>,
    #[clap(short = 'O', long)]
    /// An output file to write the trace to, otherwise stdout is used
    pub output_file: Option<PathBuf>,
    #[clap(short = 'L', long, default_value_t = LevelFilter::INFO)]
    /// The log level (error, warn, info, debug, trace)
    pub log_level: LevelFilter,
    /// The program to run
    #[clap()]
    pub program: PathBuf,
    /// The arguments to the program
    #[clap(num_args = 1.., last = true)]
    pub args: Vec<String>,
}

impl Args {
    fn to_plugin_args(&self) -> String {
        let res = format!(
            "log_insns={},log_mem={},log_syscalls={}",
            self.log_insns | self.log_all,
            self.log_mem | self.log_all,
            self.log_syscalls | self.log_all,
        );
        #[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
        let res = format!("{res},log_registers={}", self.log_registers | self.log_all);
        #[cfg(not(any(
            feature = "plugin-api-v0",
            feature = "plugin-api-v1",
            feature = "plugin-api-v2",
            feature = "plugin-api-v3",
            feature = "plugin-api-v4",
            feature = "plugin-api-v5",
        )))]
        let res = format!("{res},log_discons={}", self.log_discons | self.log_all);
        res
    }

    fn to_qemu_args(&self, socket_path: &Path, plugin_path: &Path) -> Result<Vec<String>> {
        let mut qemu_args = vec![
            "-plugin".to_string(),
            format!(
                "{},{},socket_path={}",
                plugin_path.display(),
                self.to_plugin_args(),
                socket_path.display()
            ),
            "--".to_string(),
            self.program
                .to_str()
                .ok_or_else(|| anyhow!("Failed to convert program path to string"))?
                .to_string(),
        ];

        qemu_args.extend(self.args.clone());

        Ok(qemu_args)
    }
}

async fn run<P>(qemu: P, input: Option<Vec<u8>>, args: Vec<String>) -> Result<()>
where
    P: AsRef<Path>,
{
    let mut exe = Command::new(qemu.as_ref())
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::inherit()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    debug!("Started QEMU with PID {}", exe.id());

    if let Some(input) = input {
        let mut stdin = exe.stdin.take().ok_or_else(|| anyhow!("No stdin"))?;
        spawn_blocking(move || {
            debug!("Writing input to QEMU stdin");
            stdin.write_all(&input)
        });
    }

    let stdout = exe.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;

    let out_reader = spawn_blocking(move || {
        debug!("Reading output from QEMU stdout");

        let mut line = String::new();
        let mut out_reader = BufReader::new(stdout);

        loop {
            line.clear();

            if let 0 = out_reader.read_line(&mut line)? {
                break;
            }

            let line = line.trim();

            if !line.is_empty() {
                println!("{line}");
            }
        }

        Ok::<(), Error>(())
    });

    let stderr = exe.stderr.take().ok_or_else(|| anyhow!("No stderr"))?;

    let err_reader = spawn_blocking(move || {
        debug!("Reading output from QEMU stderr");

        let mut line = String::new();
        let mut err_reader = BufReader::new(stderr);

        loop {
            line.clear();

            if let 0 = err_reader.read_line(&mut line)? {
                break;
            }

            let line = line.trim();

            if !line.is_empty() {
                eprintln!("{line}");
            }
        }

        Ok::<(), Error>(())
    });

    let waiter = spawn_blocking(move || {
        debug!("Waiting for QEMU to exit");
        exe.wait()
    });

    debug!("Waiting for tasks to complete");

    let (out_res, err_res, waiter_res) = join!(out_reader, err_reader, waiter);

    debug!("All tasks completed");

    out_res??;
    err_res??;
    waiter_res??;

    Ok(())
}

fn listen<P>(listen_sock: UnixListener, outfile: Option<P>) -> Result<()>
where
    P: AsRef<Path>,
{
    let mut outfile_stream = if let Some(outfile) = outfile.as_ref() {
        Box::new(OpenOptions::new().create(true).append(true).open(outfile)?) as Box<dyn Write>
    } else {
        Box::new(stdout()) as Box<dyn Write>
    };

    let (mut stream, _) = listen_sock.accept()?;
    let it = Deserializer::from_reader(&mut stream).into_iter::<Event>();

    for event in it {
        outfile_stream.write_all(to_string(&event?)?.as_bytes())?;
        outfile_stream.write_all(b"\n")?;
    }

    Ok(())
}

#[main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let subscriber = fmt()
        .with_writer(std::io::stderr)
        .with_max_level(args.log_level)
        .finish();

    set_global_default(subscriber)?;

    debug!("{args:?}");

    let socket_path = tmp("/tmp/qemu-", ".sock");

    let input = if let Some(input_file) = args.input_file.as_ref() {
        let Ok(input_file) = input_file.canonicalize() else {
            return Err(anyhow!("Failed to canonicalize input file"));
        };

        Some(read(input_file).await?)
    } else {
        None
    };

    debug!("Binding to socket {}", socket_path.display());

    let listen_sock = UnixListener::bind(&socket_path)?;

    let qemu_args = args.to_qemu_args(&socket_path, &args.plugin_path)?;

    let socket_task = spawn_blocking(move || {
        debug!("Listening for events on socket");
        listen(listen_sock, args.output_file.as_ref())
    });

    let qemu_task = spawn(async move {
        debug!("Running QEMU with args: {:?}", qemu_args);
        run(&args.qemu_bin, input, qemu_args).await
    });

    debug!("Waiting for QEMU and socket tasks to complete");

    let (qemu_res, socket_res) = join!(socket_task, qemu_task);

    remove_file(&socket_path).await?;

    qemu_res??;
    socket_res??;

    Ok(())
}
