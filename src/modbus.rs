use anyhow::Result;
use log::{error, info};
use std::{
    collections::HashMap,
    future::{Ready, ready},
    net::SocketAddr,
    slice,
    sync::{Arc, LazyLock, Mutex},
};
use tokio::net::TcpListener;
use tokio_modbus::{
    prelude::*,
    server::{
        Service,
        tcp::{Server, accept_tcp_connection},
    },
};

static SOCKET_ADDR: LazyLock<SocketAddr> = LazyLock::new(|| "0.0.0.0:5502".parse().unwrap());

pub(super) async fn server() -> Result<()> {
    let server = Server::new(TcpListener::bind(*SOCKET_ADDR).await?);
    let new_service = |_socket_addr| Ok(Some(ExampleService::new()));
    let on_connected = |stream, socket_addr| async move {
        accept_tcp_connection(stream, socket_addr, new_service)
    };
    let on_process_error = |error| error!("{error}");
    server.serve(&on_connected, on_process_error).await?;
    Ok(())
}

struct ExampleService {
    input_registers: Arc<Mutex<HashMap<u16, u16>>>,
    holding_registers: Arc<Mutex<HashMap<u16, u16>>>,
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
            input_registers: Arc::new(Mutex::new(input_registers)),
            holding_registers: Arc::new(Mutex::new(holding_registers)),
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
            Request::ReadInputRegisters(address, count) => {
                info!("Read input registers {address} {count}");
                read_register(&self.input_registers.lock().unwrap(), address, count)
                    .map(Response::ReadInputRegisters)
            }
            Request::ReadHoldingRegisters(address, count) => {
                read_register(&self.holding_registers.lock().unwrap(), address, count)
                    .map(Response::ReadHoldingRegisters)
            }
            Request::WriteMultipleRegisters(address, values) => write_register(
                &mut self.holding_registers.lock().unwrap(),
                address,
                &values,
            )
            .map(|_| Response::WriteMultipleRegisters(address, values.len() as u16)),
            Request::WriteSingleRegister(address, value) => write_register(
                &mut self.holding_registers.lock().unwrap(),
                address,
                slice::from_ref(&value),
            )
            .map(|_| Response::WriteSingleRegister(address, value)),
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

struct Registers {
    input: Arc<Mutex<HashMap<u16, u16>>>,
    holding: Arc<Mutex<HashMap<u16, u16>>>,
}

impl Registers {
    fn new() -> Self {
        let mut input = HashMap::new();
        input.insert(0, 1234);
        input.insert(1, 5678);
        let mut holding = HashMap::new();
        holding.insert(0, 10);
        holding.insert(1, 20);
        holding.insert(2, 30);
        holding.insert(3, 40);
        Self {
            input: Arc::new(Mutex::new(input)),
            holding: Arc::new(Mutex::new(holding)),
        }
    }

    /// Helper function implementing reading registers from a HashMap.
    fn read(
        registers: &HashMap<u16, u16>,
        address: u16,
        count: u16,
    ) -> Result<Vec<u16>, ExceptionCode> {
        let mut buffer = vec![0; count as _];
        for index in 0..count {
            let register_address = address + index;
            if let Some(register) = registers.get(&register_address) {
                buffer[index as usize] = *register;
            } else {
                error!("SERVER: Exception::IllegalDataAddress");
                return Err(ExceptionCode::IllegalDataAddress);
            }
        }
        Ok(buffer)
    }
}

/// Helper function implementing reading registers from a HashMap.
fn read_register(
    registers: &HashMap<u16, u16>,
    address: u16,
    count: u16,
) -> Result<Vec<u16>, ExceptionCode> {
    let mut buffer = vec![0; count as _];
    for index in 0..count {
        let register_address = address + index;
        if let Some(register) = registers.get(&register_address) {
            buffer[index as usize] = *register;
        } else {
            error!("SERVER: Exception::IllegalDataAddress");
            return Err(ExceptionCode::IllegalDataAddress);
        }
    }
    Ok(buffer)
}

/// Write a holding register. Used by both the write single register and write
/// multiple registers requests.
fn write_register(
    registers: &mut HashMap<u16, u16>,
    address: u16,
    values: &[u16],
) -> Result<(), ExceptionCode> {
    for (index, value) in values.iter().enumerate() {
        let register_address = address + index as u16;
        if let Some(register) = registers.get_mut(&register_address) {
            *register = *value;
        } else {
            error!("SERVER: Exception::IllegalDataAddress");
            return Err(ExceptionCode::IllegalDataAddress);
        }
    }
    Ok(())
}
