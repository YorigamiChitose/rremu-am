//! rremu-report：解析 rremu 的 dump JSON（+ 可选 ELF），输出人类可读崩溃报告。
//!
//! 用法：`rremu-report <dump.json> [--elf <path>]`
//!
//! 报告包含：终止原因、trap 详情（mcause 名称化）、寄存器（符号化）、
//! 指令 trace（反汇编），全程不依赖串口输出。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rremu_machine::dump::{Dump, EventItem, Reason};

fn main() {
    if let Err(e) = run() {
        eprintln!("rremu-report: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut dump_path: Option<PathBuf> = None;
    let mut elf_override: Option<PathBuf> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("usage: rremu-report <dump.json> [--elf <path>]");
                return Ok(());
            }
            "--elf" => {
                elf_override = Some(PathBuf::from(args.next().context("--elf requires a path")?));
            }
            _ if dump_path.is_none() => dump_path = Some(PathBuf::from(a)),
            _ => anyhow::bail!("unexpected argument: {a}"),
        }
    }
    let dump_path = dump_path.context("usage: rremu-report <dump.json> [--elf <path>]")?;
    let text = std::fs::read_to_string(&dump_path)
        .with_context(|| format!("read {}", dump_path.display()))?;
    let dump: Dump = Dump::from_json(&text).context("parse dump json")?;
    check_version(dump.format_version)?;

    let elf_path = elf_override.or_else(|| dump.elf.as_ref().map(|e| PathBuf::from(&e.path)));
    let symbols = match &elf_path {
        Some(p) => load_symbols(p).unwrap_or_default(),
        None => Vec::new(),
    };

    warn_on_elf_mismatch(&dump, elf_path.as_deref());
    print_report(&dump, &symbols);
    Ok(())
}

/// 校验 dump 格式版本（不匹配则失败，避免静默误读）。
fn check_version(version: u32) -> Result<()> {
    let supported = rremu_machine::dump::FORMAT_VERSION;
    if version != supported {
        anyhow::bail!("unsupported dump format_version {version} (this tool supports {supported})");
    }
    Ok(())
}

/// ELF 与 dump 记录的 sha256 不符时告警（符号可能错位）。
fn warn_on_elf_mismatch(dump: &Dump, elf_path: Option<&Path>) {
    let Some(expected) = dump.elf.as_ref().and_then(|e| e.sha256.as_deref()) else {
        return;
    };
    let Some(p) = elf_path else { return };
    let Some(actual) = rremu_machine::dump::ElfRef::from_path(&p.to_string_lossy()).sha256 else {
        return;
    };
    if actual != expected {
        eprintln!(
            "warning: ELF sha256 mismatch (dump={expected}, file={actual}); symbols may be inaccurate"
        );
    }
}

/// 加载 ELF 符号（地址, 名称），按地址升序。
fn load_symbols(path: &Path) -> Result<Vec<(u64, String)>> {
    let bytes = std::fs::read(path).with_context(|| format!("read ELF {}", path.display()))?;
    let elf = goblin::elf::Elf::parse(&bytes).context("parse ELF")?;
    let mut syms: Vec<(u64, String)> = elf
        .syms
        .iter()
        .filter(|s| s.st_value != 0)
        .filter_map(|s| {
            let name = elf.strtab.get_at(s.st_name)?;
            if name.is_empty() {
                None
            } else {
                Some((s.st_value, name.to_owned()))
            }
        })
        .collect();
    syms.sort_by_key(|(a, _)| *a);
    syms.dedup_by_key(|(a, _)| *a);
    Ok(syms)
}

/// 地址符号化：` <name+0xoff>`（无匹配则空串）。
fn symbolize(symbols: &[(u64, String)], addr: u64) -> String {
    let idx = symbols.partition_point(|(a, _)| *a <= addr);
    if idx == 0 {
        return String::new();
    }
    let (base, name) = &symbols[idx - 1];
    if *base == addr {
        format!(" <{name}>")
    } else {
        format!(" <{name}+0x{:x}>", addr - base)
    }
}

fn parse_hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0)
}

fn parse_hex_u64(s: &str) -> u64 {
    u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0)
}

fn disasm(pc: u32, bits: u32, len: u8) -> String {
    let r = if len == 2 {
        rremu_decode::decode16(bits as u16, pc)
    } else {
        rremu_decode::decode32(bits, pc)
    };
    match r {
        Ok(i) => i.to_string(),
        Err(_) => format!(".word {bits:#010x}"),
    }
}

fn print_report(d: &Dump, symbols: &[(u64, String)]) {
    println!("=== rremu dump report ===");
    println!("rremu   : {}", d.rremu_version);
    println!("format  : {}", d.format_version);
    if let Some(elf) = &d.elf {
        println!("elf     : {}", elf.path);
    }

    // 终止原因（退出码经 rremu-am 语义化）
    match &d.reason {
        Reason::Halt { code } => {
            let desc = rremu_am::exit_code_desc(*code)
                .map(|s| format!(" — {s}"))
                .unwrap_or_default();
            println!("reason  : halt (code={code}){desc}");
        }
        Reason::Faulted { reason } => println!("reason  : faulted — {reason}"),
        Reason::Paused => println!("reason  : paused (no exit)"),
    }

    // trap 详情
    if let Some(t) = &d.trap {
        let cause = parse_hex(&t.cause);
        println!();
        println!("trap    : {}", rremu_isa::cause::cause_name(cause));
        println!("  mcause: {}", t.cause);
        println!(
            "  mepc  : {}{}",
            t.pc,
            symbolize(symbols, parse_hex_u64(&t.pc))
        );
        println!("  mtval : {}", t.mtval);
    }

    // 寄存器
    println!();
    println!("registers:");
    for (i, v) in d.machine.gprs.iter().enumerate() {
        let sym = symbolize(symbols, parse_hex_u64(v));
        println!("  x{i:<2} {:<4} = {v}{sym}", rremu_isa::reg_name(i as u8));
    }
    println!(
        "  pc      = {}{}",
        d.machine.pc,
        symbolize(symbols, parse_hex_u64(&d.machine.pc))
    );
    println!(
        "  priv={} cycle={} instret={}",
        d.machine.priv_, d.machine.cycle, d.machine.instret
    );

    // 指令 trace（反汇编）
    if !d.trace.is_empty() {
        println!();
        println!("trace (oldest first):");
        let fault_pc = d.trap.as_ref().map(|t| parse_hex_u64(&t.pc));
        for t in &d.trace {
            let pc = parse_hex(&t.pc);
            let mark = if Some(u64::from(pc)) == fault_pc {
                "  <-- fault"
            } else {
                ""
            };
            println!(
                "  {pc:#010x}: {}{mark}",
                disasm(pc, parse_hex(&t.bits), t.len)
            );
        }
    }

    // IO 事件
    if !d.events.is_empty() {
        println!();
        println!("events (last {}):", d.events.len());
        for e in &d.events {
            match e {
                EventItem::Mem {
                    cycle,
                    kind,
                    size,
                    addr,
                    value,
                    device,
                } => {
                    println!("  [{cycle}] {kind} {size} {addr} = {value} ({device})")
                }
                EventItem::Trap { cause, pc } => {
                    println!("  trap cause={cause} pc={pc}")
                }
                EventItem::UartOut { byte } => println!("  uart out {:?}", *byte as char),
                EventItem::UartIn { byte } => println!("  uart in  {:?}", *byte as char),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_check() {
        assert!(check_version(rremu_machine::dump::FORMAT_VERSION).is_ok());
        assert!(check_version(rremu_machine::dump::FORMAT_VERSION + 1).is_err());
    }

    #[test]
    fn symbolize_nearest() {
        let syms = vec![
            (0x8000_0000u64, "_start".to_string()),
            (0x8000_00f2, "rust_main".to_string()),
        ];
        assert_eq!(symbolize(&syms, 0x8000_0000), " <_start>");
        assert_eq!(symbolize(&syms, 0x8000_0100), " <rust_main+0xe>");
        assert_eq!(symbolize(&syms, 0x7fff_0000), ""); // 低于首个符号
    }

    #[test]
    fn disasm_and_parse() {
        assert_eq!(parse_hex("0x800000f2"), 0x8000_00f2);
        assert_eq!(parse_hex_u64("0x80fffff0"), 0x80ff_fff0);
        // addi x0, x0, 0 (nop)
        assert_eq!(disasm(0x8000_0000, 0x0000_0013, 4), "addi zero, zero, 0");
        // 非法短编码回退 .word
        assert_eq!(disasm(0x0, 0x0000, 2), ".word 0x00000000");
    }
}
