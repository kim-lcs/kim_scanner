use crate::{Network, Serial};

#[derive(Clone, Debug)]
pub enum Connector {
    Serial(Serial),
    Network(Network),
}

impl Connector {
    /// 自定义转字符串
    ///
    /// # Examples
    /// ```
    /// use scanner::prelude::*;
    ///
    /// let mut conn:Connector = Network::new_server("127.0.0.1", 5000).into();
    /// assert_eq!(conn.to_string(), "127.0.0.1:5000");
    ///
    /// let mut conn:Connector = Serial::new("COM1", 9600, 8, StopBits::One, Parity::None).into();
    /// assert_eq!(conn.to_string(), "COM1");
    ///
    /// ```
    pub fn to_string(&self) -> String {
        match self {
            Connector::Serial(serial) => format!("{}", serial.name()),
            Connector::Network(network) => format!("{}:{}", network.ip(), network.port()),
        }
    }
}

impl From<Serial> for Connector {
    fn from(value: Serial) -> Self {
        Connector::Serial(value)
    }
}

impl From<Network> for Connector {
    fn from(value: Network) -> Self {
        Connector::Network(value)
    }
}
