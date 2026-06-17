use anyhow::anyhow;
#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3"
)))]
use qemu_plugin::qemu_plugin_read_memory_vaddr;
use qemu_plugin::{
    Args, Error, HasCallbacks, Info, Instruction, MemRW, MemoryInfo, PluginId, Register, Result,
    TranslationBlock, VCPUIndex, Value, register,
};
#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
use qemu_plugin::{RegisterDescriptor, qemu_plugin_get_registers};
use serde_cbor::to_writer;
use std::{
    collections::HashMap,
    os::unix::net::UnixStream,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tracer_events::{Event, InstructionEvent, MemoryEvent, SyscallEvent, SyscallSource};
use typed_builder::TypedBuilder;
use yaxpeax_x86::amd64::InstDecoder;

trait FromInstruction {
    fn from_instruction(ins: &Instruction) -> Result<Self>
    where
        Self: Sized;
}

impl FromInstruction for InstructionEvent {
    fn from_instruction(value: &Instruction) -> Result<Self> {
        let data = value.data();
        let decoder = InstDecoder::default();
        let disas = decoder
            .decode_slice(&data)
            .map(|d| d.to_string())
            .or_else(|_| value.disas())?;

        Ok(Self::builder()
            .vaddr(value.vaddr())
            .haddr(value.haddr())
            .disas(disas)
            .symbol(value.symbol()?)
            .data(data)
            .build())
    }
}

trait FromMemoryInfoVaddr {
    fn from_memory_info_vaddr(info: &MemoryInfo, vaddr: u64) -> Result<Self>
    where
        Self: Sized;
}

impl FromMemoryInfoVaddr for MemoryEvent {
    fn from_memory_info_vaddr(value: &MemoryInfo, vaddr: u64) -> Result<Self> {
        let haddr = value.hwaddr(vaddr);
        Ok(Self::builder()
            .vaddr(vaddr)
            .haddr(haddr.as_ref().map(|h| h.hwaddr()))
            .haddr_is_io(haddr.as_ref().map(|h| h.is_io()))
            .haddr_device_name(haddr.and_then(|h| h.device_name().ok().flatten()))
            .size_shift(value.size_shift())
            .size_bytes(match value.size_shift() {
                0 => 1,
                1 => 2,
                2 => 4,
                3 => 8,
                _ => 0,
            })
            .sign_extended(value.sign_extended())
            .is_store(value.is_store())
            .big_endian(value.big_endian())
            .build())
    }
}

#[derive(TypedBuilder, Clone, Debug)]
struct Tracer {
    #[builder(default)]
    pub target_name: Option<String>,
    pub syscalls: Arc<Mutex<HashMap<SyscallSource, SyscallEvent>>>,
    #[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
    pub registers: Arc<Mutex<Vec<RegisterDescriptor<'static>>>>,
    #[builder(default)]
    pub tx: Arc<Mutex<Option<UnixStream>>>,
    #[builder(default)]
    pub log_insns: bool,
    #[builder(default)]
    pub log_mem: bool,
    #[builder(default)]
    pub log_syscalls: bool,
    #[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
    #[builder(default)]
    pub log_registers: bool,
    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3",
        feature = "plugin-api-v4",
        feature = "plugin-api-v5",
    )))]
    #[builder(default)]
    pub log_discons: bool,
}

impl Tracer {
    pub fn new() -> Self {
        #[cfg(any(feature = "plugin-api-v0", feature = "plugin-api-v1"))]
        {
            Self::builder()
                .syscalls(Arc::new(Mutex::new(HashMap::new())))
                .build()
        }
        #[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
        {
            Self::builder()
                .syscalls(Arc::new(Mutex::new(HashMap::new())))
                .registers(Arc::new(Mutex::new(Vec::new())))
                .build()
        }
    }
}

impl HasCallbacks for Tracer {
    #[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
    fn on_vcpu_init(&mut self, _id: PluginId, _vcpu_id: VCPUIndex) -> Result<()> {
        *self
            .registers
            .lock()
            .map_err(|e| anyhow!("Failed to lock registers: {}", e))? =
            qemu_plugin_get_registers()?;
        Ok(())
    }

    fn on_translation_block_translate(
        &mut self,
        _id: PluginId,
        tb: TranslationBlock,
    ) -> Result<()> {
        tb.instructions().try_for_each(|insn| {
            let event = InstructionEvent::from_instruction(&insn)?;

            #[cfg(any(feature = "plugin-api-v0", feature = "plugin-api-v1"))]
            if self.log_insns {
                let tx = self.tx.clone();

                insn.register_execute_callback(move |_| {
                    tx.lock()
                        .map_err(|e| anyhow!("Failed to lock tx: {}", e))
                        .and_then(|tx| {
                            to_writer(
                                tx.as_ref().ok_or_else(|| anyhow!("No tx"))?,
                                &Event::Instruction {
                                    event: event.clone(),
                                    registers: Default::default(),
                                },
                            )
                            .map_err(|e| anyhow!(e))
                        })
                        .expect("Failed to send instruction event");
                });
            }

            #[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
            if self.log_insns {
                use qemu_plugin::CallbackFlags;

                let tx = self.tx.clone();
                let registers = self
                    .registers
                    .lock()
                    .map_err(|e| anyhow!("Failed to lock registers: {}", e))?
                    .clone();

                insn.register_execute_callback_flags(
                    move |_| {
                        tx.lock()
                            .map_err(|e| anyhow!("Failed to lock tx: {}", e))
                            .and_then(|tx| {
                                use tracer_events::{Event, Registers};

                                to_writer(
                                    tx.as_ref().ok_or_else(|| anyhow!("No tx"))?,
                                    &Event::Instruction {
                                        event: event.clone(),
                                        registers: Registers(
                                            registers
                                                .iter()
                                                .map(|r| {
                                                    let value = r.read().unwrap_or_else(|_| vec![]);
                                                    (r.name.clone(), value)
                                                })
                                                .collect(),
                                        ),
                                    },
                                )
                                .map_err(|e| anyhow!(e))
                            })
                            .expect("Failed to send instruction event");
                    },
                    CallbackFlags::QEMU_PLUGIN_CB_R_REGS,
                );
            }

            if self.log_mem {
                let tx = self.tx.clone();

                insn.register_memory_access_callback(
                    move |_, info, vaddr| {
                        tx.lock()
                            .map_err(|e| anyhow!("Failed to lock tx: {}", e))
                            .and_then(|tx| {
                                to_writer(
                                    tx.as_ref().ok_or_else(|| anyhow!("No tx"))?,
                                    &Event::Memory(MemoryEvent::from_memory_info_vaddr(
                                        &info, vaddr,
                                    )?),
                                )
                                .map_err(|e| anyhow!(e))
                            })
                            .expect("Failed to send memory event");
                    },
                    MemRW::QEMU_PLUGIN_MEM_RW,
                );
            }

            Ok::<(), Error>(())
        })?;

        Ok(())
    }

    fn on_syscall(
        &mut self,
        id: PluginId,
        vcpu_index: VCPUIndex,
        num: i64,
        a1: u64,
        a2: u64,
        a3: u64,
        a4: u64,
        a5: u64,
        a6: u64,
        a7: u64,
        a8: u64,
    ) -> Result<()> {
        if !self.log_syscalls {
            return Ok(());
        }

        #[cfg(any(
            feature = "plugin-api-v0",
            feature = "plugin-api-v1",
            feature = "plugin-api-v2",
            feature = "plugin-api-v3"
        ))]
        let event = SyscallEvent::builder()
            .num(num)
            .return_value(-1)
            .args([a1, a2, a3, a4, a5, a6, a7, a8])
            .build();

        #[cfg(not(any(
            feature = "plugin-api-v0",
            feature = "plugin-api-v1",
            feature = "plugin-api-v2",
            feature = "plugin-api-v3"
        )))]
        let event = {
            let buffers = if let Some(write_sysno) = match self.target_name.as_deref() {
                Some("i386") => Some(4),
                Some("x86_64") => Some(1),
                Some("arm") => Some(4),
                Some("aarch64") => Some(64),
                _ => None,
            } {
                if num == write_sysno {
                    let addr = a2;
                    let len = a3 as usize;
                    let mut buffer = vec![0; len];
                    qemu_plugin_read_memory_vaddr(addr, &mut buffer)?;
                    [(1, buffer)].into_iter().collect::<HashMap<_, _>>()
                } else {
                    Default::default()
                }
            } else {
                Default::default()
            };

            SyscallEvent::builder()
                .num(num)
                .return_value(-1)
                .args([a1, a2, a3, a4, a5, a6, a7, a8])
                .buffers(buffers)
                .build()
        };

        let mut syscalls = self
            .syscalls
            .lock()
            .map_err(|e| anyhow!("Failed to lock syscalls: {e}"))?;

        syscalls.insert(
            SyscallSource::builder()
                .plugin_id(id)
                .vcpu_index(vcpu_index)
                .build(),
            event,
        );

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_syscall_return(
        &mut self,
        id: PluginId,
        vcpu_index: VCPUIndex,
        num: i64,
        ret: i64,
    ) -> Result<()> {
        if !self.log_syscalls {
            return Ok(());
        }

        let mut syscalls = self
            .syscalls
            .lock()
            .map_err(|e| anyhow!("Failed to lock syscalls: {e}"))?;

        // Remove and return the syscall event
        let mut event = syscalls
            .remove(
                &SyscallSource::builder()
                    .plugin_id(id)
                    .vcpu_index(vcpu_index)
                    .build(),
            )
            .ok_or_else(|| anyhow!("No syscall event found"))?;

        #[cfg(not(any(
            feature = "plugin-api-v0",
            feature = "plugin-api-v1",
            feature = "plugin-api-v2",
            feature = "plugin-api-v3"
        )))]
        {
            if let Some(read_sysno) = match self.target_name.as_deref() {
                Some("i386") => Some(3),
                Some("x86_64") => Some(0),
                Some("arm") => Some(3),
                Some("aarch64") => Some(63),
                _ => None,
            } && num == read_sysno
            {
                let addr = event.args[1];
                let len = event.args[2] as usize;
                let mut buffer = vec![0; len];
                qemu_plugin_read_memory_vaddr(addr, &mut buffer)?;
                event.buffers.insert(1, buffer);
            }
        }

        // Update the return value
        event.return_value = ret;

        // Send the event
        let tx = self
            .tx
            .lock()
            .map_err(|e| anyhow!("Failed to lock tx: {e}"))?;
        let tx_stream = tx.as_ref().ok_or_else(|| anyhow!("No tx"))?;

        to_writer(tx_stream, &Event::Syscall(event)).map_err(|e| anyhow!(e))?;

        Ok(())
    }

    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3",
        feature = "plugin-api-v4",
        feature = "plugin-api-v5",
    )))]
    fn on_vcpu_interrupt(
        &mut self,
        _id: PluginId,
        _vcpu_id: VCPUIndex,
        from_pc: u64,
        to_pc: u64,
    ) -> Result<()> {
        if self.log_discons {
            let event = tracer_events::Interrupt {
                vaddr: from_pc,
                handler: to_pc,
            };
            self.tx
                .lock()
                .map_err(|e| anyhow!("Failed to lock tx: {}", e))
                .and_then(|tx| {
                    to_writer(
                        tx.as_ref().ok_or_else(|| anyhow!("No tx"))?,
                        &Event::Interrupt(event),
                    )
                    .map_err(|e| anyhow!(e))
                })?
        }
        Ok(())
    }

    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3",
        feature = "plugin-api-v4",
        feature = "plugin-api-v5",
    )))]
    fn on_vcpu_exception(
        &mut self,
        _id: PluginId,
        _vcpu_id: VCPUIndex,
        from_pc: u64,
        to_pc: u64,
    ) -> Result<()> {
        if self.log_discons {
            let event = tracer_events::Exception {
                epc: from_pc,
                handler: to_pc,
            };
            self.tx
                .lock()
                .map_err(|e| anyhow!("Failed to lock tx: {}", e))
                .and_then(|tx| {
                    to_writer(
                        tx.as_ref().ok_or_else(|| anyhow!("No tx"))?,
                        &Event::Exception(event),
                    )
                    .map_err(|e| anyhow!(e))
                })?
        }
        Ok(())
    }

    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3",
        feature = "plugin-api-v4",
        feature = "plugin-api-v5",
    )))]
    fn on_vcpu_hostcall(
        &mut self,
        _id: PluginId,
        _vcpu_id: VCPUIndex,
        from_pc: u64,
        to_pc: u64,
    ) -> Result<()> {
        if self.log_discons {
            let event = tracer_events::HostCall {
                vaddr: from_pc,
                next: to_pc,
            };
            self.tx
                .lock()
                .map_err(|e| anyhow!("Failed to lock tx: {}", e))
                .and_then(|tx| {
                    to_writer(
                        tx.as_ref().ok_or_else(|| anyhow!("No tx"))?,
                        &Event::HostCall(event),
                    )
                    .map_err(|e| anyhow!(e))
                })?
        }
        Ok(())
    }
}

#[derive(TypedBuilder, Clone, Debug)]
pub struct PluginArgs {
    pub log_insns: bool,
    pub log_mem: bool,
    pub log_syscalls: bool,
    #[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
    pub log_registers: bool,
    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3",
        feature = "plugin-api-v4",
        feature = "plugin-api-v5",
    )))]
    pub log_discons: bool,
    pub socket_path: PathBuf,
}

impl TryFrom<&Args> for PluginArgs {
    type Error = Error;

    fn try_from(value: &Args) -> Result<Self> {
        let builder = Self::builder()
            .log_insns(
                value
                    .parsed
                    .get("log_insns")
                    .map(|li| if let Value::Bool(v) = li { *v } else { false })
                    .unwrap_or_default(),
            )
            .log_mem(
                value
                    .parsed
                    .get("log_mem")
                    .map(|lm| if let Value::Bool(v) = lm { *v } else { false })
                    .unwrap_or_default(),
            )
            .log_syscalls(
                value
                    .parsed
                    .get("log_syscalls")
                    .map(|ls| if let Value::Bool(v) = ls { *v } else { false })
                    .unwrap_or_default(),
            );
        #[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
        let builder = builder.log_registers(
            value
                .parsed
                .get("log_registers")
                .map(|lr| if let Value::Bool(v) = lr { *v } else { false })
                .unwrap_or_default(),
        );
        #[cfg(not(any(
            feature = "plugin-api-v0",
            feature = "plugin-api-v1",
            feature = "plugin-api-v2",
            feature = "plugin-api-v3",
            feature = "plugin-api-v4",
            feature = "plugin-api-v5",
        )))]
        let builder = builder.log_discons(
            value
                .parsed
                .get("log_discons")
                .map(|lr| if let Value::Bool(v) = lr { *v } else { false })
                .unwrap_or_default(),
        );
        Ok(builder
            .socket_path(
                value
                    .parsed
                    .get("socket_path")
                    .and_then(|sp| {
                        if let Value::String(v) = sp {
                            Some(PathBuf::from(v))
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| anyhow!("No socket path provided"))?,
            )
            .build())
    }
}

impl Register for Tracer {
    fn register(&mut self, _: PluginId, args: &Args, info: &Info) -> Result<()> {
        let plugin_args = PluginArgs::try_from(args)?;

        self.target_name = Some(info.target_name.clone());

        self.tx = Arc::new(Mutex::new(Some(UnixStream::connect(
            plugin_args.socket_path,
        )?)));

        self.log_insns = plugin_args.log_insns;
        self.log_mem = plugin_args.log_mem;
        self.log_syscalls = plugin_args.log_syscalls;

        #[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
        {
            self.log_registers = plugin_args.log_registers;
        }

        #[cfg(not(any(
            feature = "plugin-api-v0",
            feature = "plugin-api-v1",
            feature = "plugin-api-v2",
            feature = "plugin-api-v3",
            feature = "plugin-api-v4",
            feature = "plugin-api-v5",
        )))]
        {
            self.log_discons = plugin_args.log_discons;
        }

        Ok(())
    }
}

register!(Tracer::new());
