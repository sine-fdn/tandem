use assert_cmd::prelude::CommandCargoExt;
use rand::{thread_rng, Rng};
use std::process::{Child, Command, Stdio};

const SERVER_CRATE: &str = "tandem_http_server";

pub fn compile_server() {
    println!("Compiling tandem_http_server, this might take a few minutes");
    Command::new("cargo")
        .arg("build")
        .arg("--features=bin")
        .arg("--release")
        .current_dir("../tandem_http_server")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    println!("Compilation finished");
}

pub fn with_server<F>(path: &str, test: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(&String) -> Result<(), Box<dyn std::error::Error>>,
{
    let (server, connection_string) = start_server(path)?;
    let res = test(&connection_string);
    let stop = stop_server(server);
    stop.and(res)
}

fn start_server(path: &str) -> Result<(Child, String), Box<dyn std::error::Error>> {
    let port: u16 = thread_rng().gen_range(8001..=9000);
    let port_str = port.to_string();
    let mut cmd = Command::cargo_bin(&SERVER_CRATE)?;
    let mut proc = cmd
        .current_dir(path)
        .env("ROCKET_PORT", port_str)
        .env("ROCKET_LOG_LEVEL", "off")
        .spawn()?;
    let connection_string = format!("127.0.0.1:{port}");
    for _ in 0..50 {
        if std::net::TcpStream::connect(&connection_string).is_ok() {
            return Ok((proc, format!("http://127.0.0.1:{port}")));
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    let _ = proc.kill();
    Err(StartTimeoutError {}.into())
}

fn stop_server(mut c: Child) -> Result<(), Box<dyn std::error::Error>> {
    c.kill()?;
    c.wait()?;
    Ok(())
}

#[derive(Debug)]
struct StartTimeoutError {}
impl std::fmt::Display for StartTimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Timeout while strating Tandem HTTP Server")
    }
}

impl std::error::Error for StartTimeoutError {}
