use crate::temperature::Request as TemperatureRequest;
use anyhow::Result;
use log::{error, info};
use std::{net::SocketAddr, sync::LazyLock};
use tokio::{
    net::TcpListener,
    sync::{mpsc::Sender, oneshot::channel as oneshot_channel},
};
use tokio_modbus::{
    prelude::*,
    server::{
        Service,
        tcp::{Server, accept_tcp_connection},
    },
};

static SOCKET_ADDR: LazyLock<SocketAddr> = LazyLock::new(|| "0.0.0.0:5502".parse().unwrap());

pub(super) async fn server(temperature_sender: Sender<TemperatureRequest>) -> Result<()> {
    let server = Server::new(TcpListener::bind(*SOCKET_ADDR).await?);
    let new_service = |_socket_addr| Ok(Some(ExampleService::new(temperature_sender.clone())));
    let on_connected = |stream, socket_addr| async move {
        accept_tcp_connection(stream, socket_addr, new_service)
    };
    let on_process_error = |error| error!("{error}");
    server.serve(&on_connected, on_process_error).await?;
    Ok(())
}

struct ExampleService {
    temperature_sender: Sender<TemperatureRequest>,
    // holding_registers: Arc<Mutex<HashMap<u16, u16>>>,
}

impl ExampleService {
    fn new(temperature_sender: Sender<TemperatureRequest>) -> Self {
        // let mut holding_registers = HashMap::new();
        // holding_registers.insert(0, 10);
        // holding_registers.insert(1, 20);
        // holding_registers.insert(2, 30);
        // holding_registers.insert(3, 40);
        Self {
            temperature_sender,
            // holding_registers: Arc::new(Mutex::new(holding_registers)),
        }
    }
}

impl Service for ExampleService {
    type Request = Request<'static>;
    type Response = Response;
    type Exception = ExceptionCode;
    type Future = impl Future<Output = Result<Self::Response, Self::Exception>>;

    fn call(&self, request: Self::Request) -> Self::Future {
        info!("Modbus request: {request:?}");
        let temperature_sender = self.temperature_sender.clone();
        async move {
            match request {
                Request::ReadInputRegisters(index, count) => {
                    if index % 2 != 0 || count % 2 != 0 {
                        error!("IllegalAddress {{ address: {index}, count: {count} }}");
                        return Err(ExceptionCode::IllegalDataAddress);
                    }
                    let start = index as usize / 2;
                    let end = start + count as usize / 2;
                    let (sender, receiver) = oneshot_channel();
                    if let Err(error) = temperature_sender.send((start..end, sender)).await {
                        error!("{error:?}");
                        return Err(ExceptionCode::ServerDeviceFailure);
                    };
                    let input_registers = match receiver.await {
                        Ok(Ok(temperatures)) => temperatures
                            .into_iter()
                            .flat_map(|temperature| {
                                let bytes = temperature.to_be_bytes();
                                [
                                    u16::from_be_bytes([bytes[0], bytes[1]]),
                                    u16::from_be_bytes([bytes[2], bytes[3]]),
                                ]
                            })
                            .collect(),
                        Ok(Err(error)) => {
                            error!("{error:?}");
                            return Err(error.into());
                        }
                        Err(error) => {
                            error!("{error:?}");
                            return Err(ExceptionCode::ServerDeviceFailure);
                        }
                    };
                    Ok(Response::ReadInputRegisters(input_registers))
                }
                _ => Err(ExceptionCode::IllegalFunction),
            }
        }
    }
}

// async fn call(
//     service: &ExampleService,
//     request: Request<'static>,
// ) -> Result<Response, ExceptionCode> {
//     match request {
//         Request::ReadInputRegisters(index, count) => {
//             // error!("Read input registers {error:?}");
//             // return Err(ExceptionCode::ServerDeviceFailure);
//             // return Err(ExceptionCode::IllegalFunction);
//             let index = index as usize;
//             let count = count as usize;
//             let (sender, receiver) = oneshot_channel();
//             info!("0!!!!!!!!!!!!!!!!!!!");
//             service
//                 .temperature_sender
//                 .send((index..count, sender))
//                 .await?;
//             info!("1!!!!!!!!!!!!!!!!!!!");
//             yield_now().await;
//             // yield_now().await;
//             let input_registers = receiver
//                 .await??
//                 .into_iter()
//                 .flat_map(|temperature| {
//                     let bytes = temperature.to_be_bytes();
//                     [
//                         u16::from_be_bytes([bytes[0], bytes[1]]),
//                         u16::from_be_bytes([bytes[2], bytes[3]]),
//                     ]
//                 })
//                 .collect();
//             info!("2!!!!!!!!!!!!!!!!!!!");
//             // let addresses = ADDRESSES.get().unwrap();
//             // if index < addresses.len() {}
//             // let address = &addresses[index];
//             // let temperature = thermometer.temperature(&address)?;
//             info!("input_registers: {input_registers:?}");
//             Ok(Response::ReadInputRegisters(input_registers))
//         }
//         // Request::ReadHoldingRegisters(address, count) => {
//         //     read_register(&self.holding_registers.lock().unwrap(), address, count)
//         //         .map(Response::ReadHoldingRegisters)
//         // }
//         // Request::WriteMultipleRegisters(address, values) => write_register(
//         //     &mut self.holding_registers.lock().unwrap(),
//         //     address,
//         //     &values,
//         // )
//         // .map(|_| Response::WriteMultipleRegisters(address, values.len() as u16)),
//         // Request::WriteSingleRegister(address, value) => write_register(
//         //     &mut self.holding_registers.lock().unwrap(),
//         //     address,
//         //     slice::from_ref(&value),
//         // )
//         // .map(|_| Response::WriteSingleRegister(address, value)),
//         _ => {
//             error!("Modbus server. IllegalFunction: {request:?}");
//             todo!()
//             // Err(ExceptionCode::IllegalFunction)
//         }
//     }
// }

// /// Helper function implementing reading registers from a HashMap.
// fn read_register(
//     registers: &HashMap<u16, u16>,
//     address: u16,
//     count: u16,
// ) -> Result<Vec<u16>, ExceptionCode> {
//     let mut buffer = vec![0; count as _];
//     for index in 0..count {
//         let register_address = address + index;
//         if let Some(register) = registers.get(&register_address) {
//             buffer[index as usize] = *register;
//         } else {
//             error!("Modbus server. IllegalDataAddress");
//             return Err(ExceptionCode::IllegalDataAddress);
//         }
//     }
//     Ok(buffer)
// }

// /// Write a holding register. Used by both the write single register and write
// /// multiple registers requests.
// fn write_register(
//     registers: &mut HashMap<u16, u16>,
//     address: u16,
//     values: &[u16],
// ) -> Result<(), ExceptionCode> {
//     for (index, value) in values.iter().enumerate() {
//         let register_address = address + index as u16;
//         if let Some(register) = registers.get_mut(&register_address) {
//             *register = *value;
//         } else {
//             error!("Modbus server. IllegalDataAddress");
//             return Err(ExceptionCode::IllegalDataAddress);
//         }
//     }
//     Ok(())
// }
