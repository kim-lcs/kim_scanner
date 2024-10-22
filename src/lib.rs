use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

mod connector;
mod error;
pub mod prelude;
use prelude::*;
use tokio_serial::SerialPortBuilderExt;
use tracing::{event, Level};

/// 扫码枪
#[derive(Clone)]
pub struct Scanner {
    /// 连接器参数
    pub connector: Connector,
    /// 超时时长
    timeout: Option<Duration>,
    /// 用于发送指令给扫码枪
    sender: Arc<Mutex<Sender<String>>>,
    /// 用于接收扫码枪指令
    receiver: Arc<Mutex<Receiver<String>>>,
}
unsafe impl Send for Scanner {}

type ScannerResult = Result<Result<(), ScannerError>, ScannerError>;

impl Scanner {
    /// 创建扫码枪
    ///
    /// * `connector` 连接器
    /// #Examples
    /// ```
    /// use scanner::prelude::*;
    /// let scanner = Scanner::new(Network::new_server("192.168.1.1", 6000));
    ///
    /// if let Connector::Network(nw) = scanner.connector {
    ///     assert_eq!(nw.ip(), "192.168.1.1");
    /// } else {
    ///     assert!(false, "connector is not network");
    /// }
    /// ```
    pub fn new(connector: impl Into<Connector>) -> Self {
        let (tx, rx) = mpsc::channel::<String>(100);
        Scanner {
            connector: connector.into(),
            sender: Arc::new(Mutex::new(tx)),
            receiver: Arc::new(Mutex::new(rx)),
            timeout: None,
        }
    }

    /// 设置超时时长
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// 给扫码枪发送指令（数据），一般用于反控
    pub async fn send_message(&self, cmd: String) -> ScannerResult {
        let sender = self.sender.lock().await;
        let r = sender.send(cmd).await;
        if let Err(e) = r {
            return Err(ScannerError::Comm(e.0));
        }
        Ok(Ok(()))
    }

    // 启动扫码枪
    pub async fn start(&self) -> ScannerResult {
        let conn: &Connector = &self.connector;
        let self_arc = Arc::new(self.clone());

        match conn {
            Connector::Serial(conn) => {
                if !conn.name().to_lowercase().starts_with("com") {
                    return Err(ScannerError::Param(format!(
                        "无效的串口名称,name={}",
                        conn.name()
                    )));
                }
                tokio::spawn(async move {
                    loop {
                        let self_arc = Arc::clone(&self_arc);
                        let conn: &Connector = &self_arc.connector;
                        if let Err(err) = self_arc.start_serial().await {
                            event!(
                                Level::ERROR,
                                "\t{}\t致命错误❌❌❌\t错误原因={:?}",
                                conn.to_string(),
                                err
                            );
                            break;
                        }
                        // 非致命错误等待3秒重启
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        event!(Level::INFO, "\t{}\t重新启动串口🔃", conn.to_string());
                    }
                });
            }
            Connector::Network(conn) => {
                if Ipv4Addr::from_str(conn.ip()).is_err() {
                    return Err(ScannerError::Param(format!(
                        "无效的IP地址,ip={}",
                        conn.ip()
                    )));
                }
                // 创建线程启动扫码枪
                tokio::spawn(async move {
                    loop {
                        let self_arc = Arc::clone(&self_arc);
                        let conn: &Connector = &self_arc.connector;
                        let is_server = match conn {
                            Connector::Serial(_) => false,
                            Connector::Network(conn) => conn.is_server(),
                        };
                        let r = if is_server {
                            self_arc.start_network_server().await
                        } else {
                            self_arc.start_network_client().await
                        };
                        // 出现致命错误后，返回给主线程，否则重启服务
                        if let Err(err) = r {
                            event!(
                                Level::ERROR,
                                "\t{}\t致命错误❌❌❌\t错误原因={:?}",
                                conn.to_string(),
                                err
                            );
                            break;
                        }
                    }
                });
            }
        }
        Ok(Ok(()))
    }

    /// 启动网络扫码枪`服务器模式`
    async fn start_network_server(&self) -> ScannerResult {
        // 检查参数是否一致
        let conn = match &self.connector {
            Connector::Serial(conn) => {
                let err = format!("此处应该是网络参数，但是却收到了串口参数({})", conn.name());
                return Err(ScannerError::Param(err));
            }
            Connector::Network(conn) => conn,
        };
        let receiver = Arc::clone(&self.receiver);
        let addr = format!("{}:{}", conn.ip(), conn.port());
        // 创建服务
        let server = TcpListener::bind(&addr).await;
        if let Err(err) = server {
            event!(
                Level::ERROR,
                "\t{}\t扫码枪服务创建失败❌\t失败原因={}",
                &addr,
                err
            );
            return Ok(Err(ScannerError::Io(err)));
        }
        event!(Level::INFO, "\t{}\t扫码枪服务创建成功✅", &addr);
        let server = server.unwrap();
        // 等待客户端连接
        event!(Level::INFO, "\t{}\t等待扫码枪连接⌛⌛⌛", &addr);
        let client = server.accept().await;
        if let Err(err) = client {
            event!(
                Level::ERROR,
                "\t{}\t扫码枪连接错误❌\t错误原因={}",
                &addr,
                err
            );
            return Err(ScannerError::Comm(err.to_string()));
        }
        let (client, _) = client.unwrap();
        event!(
            Level::INFO,
            "\t{}\t扫码枪连接成功✅\t扫码枪地址={:?}",
            &addr,
            &client.peer_addr().unwrap()
        );
        let (mut rx, mut tx) = client.into_split();
        // ! 读取条码线程
        let addr1 = addr.to_owned();
        let read_handle = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                let r = rx.read(&mut buf).await;
                match r {
                    Ok(n) if n == 0 => {
                        event!(Level::ERROR, "\t{}\t接收数据为空,关闭连接❌", &addr1);
                        break;
                    }
                    Ok(n) => {
                        let s = String::from_utf8_lossy(&buf[0..n]);
                        event!(Level::INFO, "\t{}\t接收条码={}", &addr1, s);
                    }
                    Err(err) => {
                        event!(
                            Level::ERROR,
                            "\t{}\t接收数据错误❌\t错误原因={:?}",
                            &addr1,
                            err
                        );
                        break;
                    }
                }
            }
        });
        // ! 发送命令线程
        let addr2 = addr.to_owned();
        let write_handle = tokio::spawn(async move {
            let mut receiver = receiver.lock().await;
            loop {
                let cmd = receiver.recv().await;
                if let Some(cmd) = cmd {
                    let buf = cmd.as_bytes();
                    let r = tx.write(buf).await;
                    if let Err(err) = r {
                        event!(
                            Level::ERROR,
                            "\t{}\t发送数据错误❌\t错误原因={:?}",
                            &addr2,
                            err
                        );
                        break;
                    }
                }
            }
        });
        if let Err(err) = read_handle.await {
            event!(
                Level::ERROR,
                "\t{}\t接收线程错误❌\t错误原因={:?}",
                &addr,
                err
            )
        }
        event!(Level::INFO, "\t{}\t接收线程关闭❌", &addr);
        write_handle.abort(); // 👈 读取线程关闭后,自动关闭写入线程
        event!(Level::INFO, "\t{}\t发送线程关闭❌", &addr);
        Ok(Ok(()))
    }

    /// 启动网络扫码枪`客户端模式`
    async fn start_network_client(&self) -> ScannerResult {
        // 检查参数是否一致
        let conn = match &self.connector {
            Connector::Serial(conn) => {
                let err = format!("此处应该是网络参数，但是却收到了串口参数({})", conn.name());
                return Err(ScannerError::Param(err));
            }
            Connector::Network(conn) => conn,
        };
        let receiver = Arc::clone(&self.receiver);
        let addr = format!("{}:{}", conn.ip(), conn.port());
        // 连接扫码枪服务
        let client = TcpStream::connect(&addr).await;
        if let Err(err) = client {
            event!(
                Level::ERROR,
                "\t{}\t扫码枪连接错误❌\t错误原因={}",
                &addr,
                err
            );
            return Ok(Err(ScannerError::Comm(err.to_string())));
        }
        let client = client.unwrap();
        event!(
            Level::INFO,
            "\t{}\t扫码枪连接成功✅\t扫码枪地址={:?}",
            &addr,
            &client.peer_addr().unwrap()
        );
        let (mut rx, mut tx) = client.into_split();
        // ! 读取条码线程
        let addr1 = addr.to_owned();
        let read_handle = tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                let r = rx.read(&mut buf).await;
                match r {
                    Ok(n) if n == 0 => {
                        event!(Level::ERROR, "\t{}\t接收数据为空,关闭连接❌", &addr1);
                        break;
                    }
                    Ok(n) => {
                        let s = String::from_utf8_lossy(&buf[0..n]);
                        event!(Level::INFO, "\t{}\t接收条码={}", &addr1, s);
                    }
                    Err(err) => {
                        event!(
                            Level::ERROR,
                            "\t{}\t接收数据错误❌\t错误原因={:?}",
                            &addr1,
                            err
                        );
                        break;
                    }
                }
            }
        });
        // ! 发送命令线程
        let addr2 = addr.to_owned();
        let write_handle = tokio::spawn(async move {
            let mut receiver = receiver.lock().await;
            loop {
                let cmd = receiver.recv().await;
                if let Some(cmd) = cmd {
                    let buf = cmd.as_bytes();
                    let r = tx.write(buf).await;
                    if let Err(err) = r {
                        event!(
                            Level::ERROR,
                            "\t{}\t发送数据错误❌\t错误原因={:?}",
                            &addr2,
                            err
                        );
                        break;
                    }
                }
            }
        });
        if let Err(err) = read_handle.await {
            event!(
                Level::ERROR,
                "\t{}\t接收线程错误❌\t错误原因={:?}",
                &addr,
                err
            )
        }
        event!(Level::INFO, "\t{}\t接收线程关闭❌", &addr);
        write_handle.abort(); // 👈 读取线程关闭后,自动关闭写入线程
        event!(Level::INFO, "\t{}\t发送线程关闭❌", &addr);
        Ok(Ok(()))
    }

    /// 启动串口扫码枪
    async fn start_serial(&self) -> ScannerResult {
        // 检查参数是否一致
        let conn = match &self.connector {
            Connector::Serial(conn) => conn,
            Connector::Network(conn) => {
                let err = format!(
                    "此处应该是串口参数，但是却收到了网络参数({}:{})",
                    conn.ip(),
                    conn.port()
                );
                return Err(ScannerError::Param(err));
            }
        };
        let addr = conn.name().to_owned();
        // let receiver = Arc::clone(&self.receiver);

        // let ports = tokio_serial::available_ports();
        // if let Err(err) = ports {
        //     event!(
        //         Level::ERROR,
        //         "\t{}\t串口查询错误❌\t错误原因={}",
        //         &addr,
        //         err
        //     );
        //     return Ok(Err(ScannerError::Comm(err.to_string())));
        // }
        // let ports = ports.unwrap();
        // println!("{:?}", ports);

        // 串口连接
        // TODO timeout 实测不起作用
        let timeout = self.timeout.unwrap_or(Duration::from_secs(60)); // 不能将下面这行拆开用条件判断，所以只能这样了
        let com = tokio_serial::new(&addr, conn.baudrate())
            .stop_bits(tokio_serial::StopBits::One)
            .parity(tokio_serial::Parity::None)
            .timeout(timeout)
            .open_native_async();
        if let Err(err) = com {
            event!(
                Level::ERROR,
                "\t{}\t串口连接错误❌\t错误原因={}\t参数={:?}",
                &addr,
                err,
                &conn
            );
            return Ok(Err(ScannerError::Comm(err.to_string())));
        }
        let mut com = com.unwrap();
        event!(Level::INFO, "\t{}\t串口连接成功✅", &conn.name());
        // 测试写入串口数据
        // let mut buf = "123456789".as_bytes();
        // let r = com.write_buf(&mut buf).await;
        // println!("串口写入：{:?}", r);
        // ! 读取串口数据
        // tokio::spawn(async move {
        loop {
            let mut buf = [0u8; 1024];
            let r = com.read(&mut buf).await;
            match r {
                Ok(n) if n == 0 => {
                    event!(Level::ERROR, "\t{}\t接收数据为空,关闭连接❌", &addr);
                    break;
                }
                Ok(n) => {
                    let barcodes = String::from_utf8_lossy(&buf[0..n]);
                    let arr: Vec<&str> = barcodes.split(&['\r', '\n'][..]).collect();
                    for barcode in arr {
                        if barcode.len() > 0 {
                            event!(Level::INFO, "\t{}\t接收条码={}", &addr, barcode);
                        }
                    }
                }
                Err(err) => {
                    event!(
                        Level::ERROR,
                        "\t{}\t接收数据错误❌\t错误原因={:?}",
                        &addr,
                        err
                    );
                    break;
                }
            }
        }
        // });
        Ok(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn new_network() {
        let conn = Connector::Network(Network::new_server("192.168.1.1", 6000).into());
        assert_eq!(conn.to_string(), "192.168.1.1:6000");
        if let Connector::Network(nw) = conn {
            assert_eq!(nw.ip(), "192.168.1.1");
        } else {
            assert!(false, "connector is not network");
        }
    }

    #[test]
    fn scanner_error() {
        let err: Result<(), ScannerError> = Err(ScannerError::Param("无效的IP地址".into()));
        assert!(err.is_err(), "err is not a scanner error");
        let err = err.unwrap_err();
        assert_eq!(err.to_string(), "扫码枪参数错误:无效的IP地址");
    }

    #[tokio::test]
    async fn start_network_server() {
        let conn = Network::new_server("127.0.0.1", 6000);
        let scanner = Scanner::new(conn);
        let r = scanner.start().await;
        assert!(r.is_ok());
    }
}
