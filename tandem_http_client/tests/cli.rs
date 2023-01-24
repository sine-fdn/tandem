#![cfg(not(target_arch = "wasm32"))]

use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use rand::prelude::*;
use std::process::{Child, Command, Stdio}; // Run programs

const CRATE_NAME: &str = "tandem_http_client";
const SERVER_CRATE: &str = "tandem_http_server";
const SERVER_URL: &str = "http://localhost:8000";

#[test]
fn file_doesnt_exist() -> Result<(), Box<dyn std::error::Error>> {
    new_command(SERVER_URL, "foobar", "main", "", "")?
        .assert()
        .failure()
        .stderr(predicate::str::contains("Could not open file"));

    Ok(())
}

#[test]
fn invalid_url() -> Result<(), Box<dyn std::error::Error>> {
    let url = "localhost";
    new_command(url, "foobar", "main", "", "")?
        .assert()
        .failure()
        .stderr(predicate::str::contains(format!(
            "invalid value \'{url}\' for '--url <URL>'"
        )));

    Ok(())
}

#[test]
fn test_too_many_parties() -> Result<(), Box<dyn std::error::Error>> {
    new_command(SERVER_URL, "tests/.manyparties.garble.rs", "main", "", "")?
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a 2-Party function"));

    Ok(())
}

#[test]
fn integration_test_and() -> Result<(), Box<dyn std::error::Error>> {
    with_server(|connection_string| {
        for _ in 0..3 {
            let party_a: u8 = random();
            let party_b: u8 = random();

            let mut cmd = new_command(
                connection_string,
                "tests/.add.garble.rs",
                "main",
                &format!("{party_a}u8"),
                &format!("{party_b}u8"),
            )?;

            if party_a as u16 + party_b as u16 > u8::MAX as u16 {
                cmd.assert()
                    .failure()
                    .stderr(predicate::str::contains("Panic due to Overflow"));
            } else {
                cmd.assert()
                    .success()
                    .stdout(predicate::str::contains(format!("{}", party_a + party_b)));
            }
        }

        Ok(())
    })
}

#[test]
fn integration_test_div_by_zero() -> Result<(), Box<dyn std::error::Error>> {
    with_server(|url| {
        let party_b: u8 = random();

        let mut cmd = new_command(
            url,
            "tests/.div.garble.rs",
            "main",
            "0u8",
            &format!("{party_b}u8"),
        )?;

        cmd.assert().failure().stderr(predicate::str::contains(
            "Panic due to Division By Zero on line 2:5",
        ));

        Ok(())
    })
}

fn new_command(
    url: &str,
    program: &str,
    function: &str,
    input: &str,
    metadata: &str,
) -> Result<Command, Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(CRATE_NAME)?;
    cmd.arg(program)
        .args(["--function", function, "--url", url])
        .arg("--input")
        .arg(input)
        .arg("--metadata")
        .arg(metadata);

    Ok(cmd)
}

fn start_server() -> Result<(Child, String), Box<dyn std::error::Error>> {
    if cfg!(not(tarpaulin)) {
        println!("Compiling tandem_http_server, this might take a few minutes");
        Command::new("cargo")
            .arg("build")
            .arg("--features=bin")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .unwrap();
        println!("Compilation finished");
    }
    let port: u16 = thread_rng().gen_range(8001..=9000);
    let port_str = port.to_string();
    let mut cmd = Command::cargo_bin(SERVER_CRATE)?;
    let mut proc = cmd
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

fn with_server<F>(test: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(&String) -> Result<(), Box<dyn std::error::Error>>,
{
    let (server, connection_string) = start_server()?;
    let res = test(&connection_string);
    let stop = stop_server(server);
    stop.and(res)
}

#[derive(Debug)]
struct StartTimeoutError {}
impl std::fmt::Display for StartTimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Timeout while strating tandem_http_server")
    }
}

impl std::error::Error for StartTimeoutError {}
