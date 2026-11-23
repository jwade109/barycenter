use std::{
    process::{Child, Command},
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::Duration,
};

use log::info;

fn run_program(cmd: &str, args: &[&str]) -> std::io::Result<(Vec<String>, Child)> {
    info!("Running program {cmd} with args {args:?}");
    let child = Command::new(cmd).args(args).spawn()?;
    let mut vec_args = vec![cmd.to_string()];
    vec_args.extend(args.into_iter().map(|s| s.to_string()));
    Ok((vec_args, child))
}

fn main() -> std::io::Result<()> {
    simple_logger::init().unwrap();

    let programs = Arc::new(Mutex::new(vec![
        run_program(
            "cargo",
            &["run", "--release", "--bin", "server", "--", "saves/"],
        )?,
        run_program("cargo", &["run", "--release", "--bin", "bary_debugger"])?,
        // run_program(
        //     "cargo",
        //     &[
        //         "run",
        //         "--release",
        //         "--bin",
        //         "client_app",
        //         "--",
        //         "--use-mp-backend",
        //         "--username",
        //         "kim_mcbudget",
        //     ],
        // )?,
    ]));

    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_clone = is_running.clone();

    ctrlc::set_handler(move || {
        info!("Got CTRL-C");

        if let Ok(mut programs) = programs.lock() {
            for (args, program) in programs.iter_mut() {
                _ = program.kill();
                info!("{args:?}: {:?}", program.wait());
            }
        }

        is_running_clone.store(false, std::sync::atomic::Ordering::Relaxed);

        std::process::exit(1);
    })
    .expect("Error setting Ctrl-C handler");

    while is_running.load(std::sync::atomic::Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(50));
    }

    info!("Done.");
    Ok(())
}
