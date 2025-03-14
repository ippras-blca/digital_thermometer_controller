use std::net::{SocketAddr, ToSocketAddrs as _};
use esp_idf_svc::{sys::EspError, timer::EspTaskTimerService};
use log::{error, info};
use tokio::{
    io,
    net::{TcpListener, TcpStream},
};
// use async_io::Async;
// use futures::{
//     AsyncReadExt, AsyncWriteExt, FutureExt,
//     executor::{LocalPool, LocalSpawner},
//     task::LocalSpawnExt,
// };

// async fn tcp_client() -> Result<(), io::Error> {
//     info!("About to open a TCP connection to 1.1.1.1 port 80");
//     let socket_addr = "one.one.one.one:80".to_socket_addrs()?.next().unwrap();
//     let mut stream = TcpStream::connect(socket_addr).await?;

//     stream.write_all("GET / HTTP/1.0\n\n".as_bytes()).await?;

//     let mut result = Vec::new();

//     stream.read_to_end(&mut result).await?;

//     info!(
//         "1.1.1.1 returned:\n=================\n{}\n=================\nSince it returned something, all is OK",
//         std::str::from_utf8(&result).map_err(|_| io::ErrorKind::InvalidData)?
//     );
//     Ok(())
// }

pub(super) async fn server() -> Result<(), io::Error> {
    async fn accept() -> Result<(), io::Error> {
        info!("About to bind a simple echo service to port 8080; do `telnet <ip-from-above>:8080`");

        // let addr = "0.0.0.0:8080".to_socket_addrs()?.next().unwrap();
        let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
        let listener = TcpListener::bind(addr).await?;
        loop {
            match listener.accept().await {
                Ok((stream, socket_addr)) => {
                    info!("Accepted client {}", socket_addr);
                    // spawner.spawn_local(handle(stream)).unwrap();
                }
                Err(error) => {
                    error!("Error: {}", error);
                }
            }
        }
    }

    // async fn handle(mut stream: Async<TcpStream>) {
    //     // read 128 bytes at a time from stream echoing back to stream
    //     loop {
    //         let mut read = [0; 128];

    //         match stream.read(&mut read).await {
    //             Ok(n) => {
    //                 if n == 0 {
    //                     // connection was closed
    //                     break;
    //                 }

    //                 let _ = stream.write_all(&read[0..n]).await;
    //             }
    //             Err(err) => {
    //                 panic!("{}", err);
    //             }
    //         }
    //     }
    // }

    accept().await
}
