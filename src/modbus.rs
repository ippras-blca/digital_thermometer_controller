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

pub(super) async fn run(temperature_sender: Sender<TemperatureRequest>) -> Result<()> {
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
}

impl ExampleService {
    fn new(temperature_sender: Sender<TemperatureRequest>) -> Self {
        Self { temperature_sender }
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
                            .flat_map(|(_address, temperature)| {
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
