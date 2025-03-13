use anyhow::Result;
use log::{error, info};
use std::{
    collections::HashMap,
    future::{Ready, ready},
    net::SocketAddr,
    time::Duration,
};
use tokio::{net::TcpListener, select, time::sleep};
use tokio_modbus::{
    prelude::*,
    server::{
        Service,
        tcp::{Server, accept_tcp_connection},
    },
};

pub(super) async fn server() -> Result<()> {
    let socket_addr = "0.0.0.0:5502".parse().unwrap();
    // select! {
    //     _ = server_context(socket_addr) => unreachable!(),
    //     _ = client_context(socket_addr) => println!("Exiting"),
    // }
    server_context(socket_addr).await?;
    Ok(())
}

async fn server_context(socket_addr: SocketAddr) -> anyhow::Result<()> {
    info!("Starting up server on {socket_addr}");
    let server = Server::new(TcpListener::bind(socket_addr).await?);
    info!("Server started");
    let new_service = |_socket_addr| Ok(Some(ExampleService::new()));
    info!("Server started");
    let on_connected = |stream, socket_addr| async move {
        error!("on_connected");
        accept_tcp_connection(stream, socket_addr, new_service)
    };
    let on_process_error = |error| {
        eprintln!("{error}");
    };
    server.serve(&on_connected, on_process_error).await?;
    Ok(())
}

async fn client_context(socket_addr: SocketAddr) {
    tokio::join!(
        async {
            // Give the server some time for starting up
            sleep(Duration::from_secs(1)).await;

            println!("CLIENT: Connecting client...");
            let mut ctx = tcp::connect(socket_addr).await.unwrap();

            println!("CLIENT: Reading 2 input registers...");
            let response = ctx.read_input_registers(0x00, 2).await.unwrap();
            println!("CLIENT: The result is '{response:?}'");
            assert_eq!(response.unwrap(), vec![1234, 5678]);

            println!("CLIENT: Writing 2 holding registers...");
            ctx.write_multiple_registers(0x01, &[7777, 8888])
                .await
                .unwrap()
                .unwrap();

            // Read back a block including the two registers we wrote.
            println!("CLIENT: Reading 4 holding registers...");
            let response = ctx.read_holding_registers(0x00, 4).await.unwrap();
            println!("CLIENT: The result is '{response:?}'");
            assert_eq!(response.unwrap(), vec![10, 7777, 8888, 40]);

            // Now we try to read with an invalid register address.
            // This should return a Modbus exception response with the code
            // IllegalDataAddress.
            println!(
                "CLIENT: Reading nonexistent holding register address... (should return IllegalDataAddress)"
            );
            let response = ctx.read_holding_registers(0x100, 1).await.unwrap();
            println!("CLIENT: The result is '{response:?}'");
            assert!(matches!(response, Err(ExceptionCode::IllegalDataAddress)));

            println!("CLIENT: Done.")
        },
        tokio::time::sleep(Duration::from_secs(5))
    );
}

struct ExampleService {
    input_registers: HashMap<u16, u16>,
    holding_registers: HashMap<u16, u16>,
}

impl ExampleService {
    fn new() -> Self {
        // Insert some test data as register values.
        let mut input_registers = HashMap::new();
        input_registers.insert(0, 1234);
        input_registers.insert(1, 5678);
        let mut holding_registers = HashMap::new();
        holding_registers.insert(0, 10);
        holding_registers.insert(1, 20);
        holding_registers.insert(2, 30);
        holding_registers.insert(3, 40);
        Self {
            input_registers,
            holding_registers,
        }
    }
}

impl Service for ExampleService {
    type Request = Request<'static>;
    type Response = Response;
    type Exception = ExceptionCode;
    type Future = Ready<Result<Self::Response, Self::Exception>>;

    fn call(&self, request: Self::Request) -> Self::Future {
        let res = match request {
            Request::ReadInputRegisters(addr, cnt) => {
                info!("Read input registers {addr} {cnt}");
                register_read(&self.input_registers, addr, cnt).map(Response::ReadInputRegisters)
            }
            Request::ReadHoldingRegisters(addr, cnt) => {
                register_read(&self.holding_registers, addr, cnt)
                    .map(Response::ReadHoldingRegisters)
            }
            // Request::WriteMultipleRegisters(addr, values) => {
            //     register_write(&mut self.holding_registers.lock().unwrap(), addr, &values)
            //         .map(|_| Response::WriteMultipleRegisters(addr, values.len() as u16))
            // }
            // Request::WriteSingleRegister(addr, value) => register_write(
            //     &mut self.holding_registers.lock().unwrap(),
            //     addr,
            //     std::slice::from_ref(&value),
            // )
            // .map(|_| Response::WriteSingleRegister(addr, value)),
            _ => {
                println!(
                    "SERVER: Exception::IllegalFunction - Unimplemented function code in request: {request:?}"
                );
                Err(ExceptionCode::IllegalFunction)
            }
        };
        ready(res)
    }
}

/// Helper function implementing reading registers from a HashMap.
fn register_read(
    registers: &HashMap<u16, u16>,
    address: u16,
    count: u16,
) -> Result<Vec<u16>, ExceptionCode> {
    let mut response_values = vec![0; count as _];
    for index in 0..count {
        let register_addr = address + index;
        if let Some(register) = registers.get(&register_addr) {
            response_values[index as usize] = *register;
        } else {
            error!("SERVER: Exception::IllegalDataAddress");
            return Err(ExceptionCode::IllegalDataAddress);
        }
    }
    Ok(response_values)
}

/// Write a holding register. Used by both the write single register and write
/// multiple registers requests.
fn register_write(
    registers: &mut HashMap<u16, u16>,
    address: u16,
    values: &[u16],
) -> Result<(), ExceptionCode> {
    for (index, value) in values.iter().enumerate() {
        let reg_addr = address + index as u16;
        if let Some(r) = registers.get_mut(&reg_addr) {
            *r = *value;
        } else {
            println!("SERVER: Exception::IllegalDataAddress");
            return Err(ExceptionCode::IllegalDataAddress);
        }
    }
    Ok(())
}
