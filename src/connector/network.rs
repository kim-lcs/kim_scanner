/// 网络连接器
#[derive(Clone, Debug)]
pub struct Network {
    ip: String,
    port: u16,
    /// 是否为服务器模式
    ///
    /// * `true` 服务器模式
    /// * `false` 客户端模式
    is_server: bool,
}

impl Network {
    /// 创建一个TCP服务器连接器
    ///
    /// * `ip` ip 地址
    /// * `port` 端口
    /// #Examples
    /// ```
    /// use scanner::prelude::*;
    ///
    /// let conn:Connector = Network::new_server("127.0.0.1", 5000).into();
    ///
    /// if let Connector::Network(conn) = conn {
    ///     assert_eq!(conn.ip(), "127.0.0.1");
    ///     assert_eq!(conn.port(), 5000);
    ///     assert_eq!(conn.is_server(), true);
    /// }
    /// ```
    pub fn new_server(ip: &str, port: u16) -> Network {
        Network {
            ip: ip.into(),
            port,
            is_server: true,
        }
    }

    /// 创建一个TCP客户端连接器
    ///
    /// * `ip` ip 地址
    /// * `port` 端口
    /// #Examples
    /// ```
    /// use scanner::prelude::*;
    ///
    /// let conn:Connector = Network::new_client("127.0.0.1", 5000).into();
    ///
    /// if let Connector::Network(conn) = conn {
    ///     assert_eq!(conn.ip(), "127.0.0.1");
    ///     assert_eq!(conn.port(), 5000);
    ///     assert_eq!(conn.is_server(), false);
    /// }
    /// ```
    pub fn new_client(ip: &str, port: u16) -> Network {
        Network {
            ip: ip.into(),
            port,
            is_server: false,
        }
    }

    /// 是否为服务器模式
    pub fn is_server(&self) -> bool {
        self.is_server
    }

    /// 获取IP地址
    pub fn ip(&self) -> &str {
        &self.ip
    }

    /// 获取端口
    pub fn port(&self) -> u16 {
        self.port
    }
}
