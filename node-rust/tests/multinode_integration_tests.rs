use std::{
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const BIN: &str = env!("CARGO_BIN_EXE_axiom-node");
const ROOT: &str = env!("CARGO_MANIFEST_DIR");

struct ChildGuard {
    name: String,
    child: Child,
}

impl ChildGuard {
    fn spawn(name: &str, config: &str) -> Self {
        let child = Command::new(BIN)
            .arg("--config")
            .arg(config)
            .current_dir(ROOT)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {name}: {e}"));

        Self {
            name: name.to_string(),
            child,
        }
    }

    fn assert_running(&mut self) {
        match self.child.try_wait().expect("try_wait failed") {
            None => {}
            Some(status) => panic!("{} exited unexpectedly: {}", self.name, status),
        }
    }

    fn kill_and_wait(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn assert_ports_available(ports: &[u16]) {
    let mut listeners = Vec::new();
    for port in ports {
        let listener = TcpListener::bind(("127.0.0.1", *port))
            .unwrap_or_else(|e| panic!("port {} is already in use before test start: {}", port, e));
        listeners.push(listener);
    }
    drop(listeners);
}

fn wait_for_port(port: u16, timeout: Duration) {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("port {} did not become reachable in time", port);
}

fn wait_for_port_closed(port: u16, timeout: Duration) {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if TcpStream::connect(("127.0.0.1", port)).is_err() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("port {} did not close in time", port);
}

#[test]
fn four_node_cluster_starts_and_recovers_node_restart() {
    let ports = [7001u16, 7002, 7003, 7004];
    assert_ports_available(&ports);

    let mut node1 = ChildGuard::spawn("node1", "examples/node1.toml");
    let mut node2 = ChildGuard::spawn("node2", "examples/node2.toml");
    let mut node3 = ChildGuard::spawn("node3", "examples/node3.toml");
    let mut node4 = ChildGuard::spawn("node4", "examples/node4.toml");

    wait_for_port(7001, Duration::from_secs(10));
    wait_for_port(7002, Duration::from_secs(10));
    wait_for_port(7003, Duration::from_secs(10));
    wait_for_port(7004, Duration::from_secs(10));

    node1.assert_running();
    node2.assert_running();
    node3.assert_running();
    node4.assert_running();

    node2.kill_and_wait();
    wait_for_port_closed(7002, Duration::from_secs(5));

    node1.assert_running();
    node3.assert_running();
    node4.assert_running();

    let mut node2_restarted = ChildGuard::spawn("node2-restarted", "examples/node2.toml");
    wait_for_port(7002, Duration::from_secs(10));

    node1.assert_running();
    node2_restarted.assert_running();
    node3.assert_running();
    node4.assert_running();
}
