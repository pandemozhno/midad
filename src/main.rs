use std::env;
use std::fs;
use std::process;

use midc::backend::{c::CBackend, js::JsBackend, Backend};
use midc::checker;
use midc::lexer::Lexer;
use midc::lower::Lowerer;
use midc::parser::Parser;
use midc::profile::Profile;

fn main() {
    if let Err(e) = run() {
        eprintln!("midc: {}", e);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let mut input = None;
    let mut output = None;
    let mut lang = "en".to_string();
    let mut backend = Backend::C;
    let mut sema_dir = "sema".to_string();

    while let Some(a) = args.next() {
        match a.as_str() {
            "--lang"    => lang = args.next().ok_or("--lang без значения")?,
            "-o"        => output = Some(args.next().ok_or("-o без значения")?),
            "--backend" => {
                let b = args.next().ok_or("--backend без значения")?;
                backend = Backend::from_str(&b).ok_or(format!("неизвестный бэкенд '{}'", b))?;
            }
            "--sema"    => sema_dir = args.next().ok_or("--sema без значения")?,
            other       => input = Some(other.to_string()),
        }
    }

    let input = input.ok_or("использование: midc <file.mid> [--lang en|ru|ar|zh] [--backend c|js] -o <out>")?;
    let output = output.ok_or("нужен -o <out>")?;

    let sema_path = format!("{}/{}.sema", sema_dir, lang);
    let profile = Profile::load(&sema_path)?;

    let src = fs::read_to_string(&input)?;
    let tokens = Lexer::new(&src, &profile).tokenize()?;
    let module = Parser::new(tokens).parse_module()?;
    let sem = Lowerer::new().lower(module)?;
    checker::check(&sem)?;

    let code = match backend {
        Backend::C  => CBackend::new().emit(&sem),
        Backend::Js => JsBackend::new().emit(&sem),
    };
    fs::write(&output, code)?;
    println!("midc: {} -> {} ({} backend)", input, output, if matches!(backend, Backend::C) { "C" } else { "JS" });
    Ok(())
}