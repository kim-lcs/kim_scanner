/// 串口连接器
#[derive(Clone, Debug)]
pub struct Serial {
    name: String,
    baudrate: u32,
    databits: u8,
    stopbits: StopBits,
    parity: Parity,
}

impl Serial {
    /// 创建串口连接器
    ///
    /// * `name` 串口名称
    /// * `baudrate` 波特率
    /// * `databits` 数据位
    /// * `stopbits` 停止位
    /// * `parity` 奇偶校验
    pub fn new(
        name: &str,
        baudrate: u32,
        databits: u8,
        stopbits: StopBits,
        parity: Parity,
    ) -> Self {
        Serial {
            name: name.into(),
            baudrate,
            databits,
            stopbits,
            parity,
        }
    }

    /// 获取串口名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取波特率
    pub fn baudrate(&self) -> u32 {
        self.baudrate
    }

    /// 获取数据位
    pub fn databits(&self) -> u8 {
        self.databits
    }

    /// 获取停止位
    pub fn stopbits(&self) -> &StopBits {
        &self.stopbits
    }

    /// 获取奇偶校验
    pub fn parity(&self) -> &Parity {
        &self.parity
    }
}

/// 奇偶校验
#[derive(Clone, Debug)]
pub enum Parity {
    /// 不发生奇偶校验检查。
    None = 0,
    /// 设置奇偶校验位，使位数等于奇数。
    Odd = 1,
    /// 设置奇偶校验位，使位数等于偶数。
    Even = 2,
    /// 将奇偶校验位保留为 1。
    Mark = 3,
    /// 将奇偶校验位保留为 0。
    Space = 4,
}

impl From<tokio_serial::Parity> for Parity {
    fn from(value: tokio_serial::Parity) -> Self {
        match value {
            tokio_serial::Parity::None => Parity::None,
            tokio_serial::Parity::Odd => Parity::Odd,
            tokio_serial::Parity::Even => Parity::Even,
        }
    }
}

// impl Into<Parity> for tokio_serial::Parity {
//     fn into(self) -> Parity {
//         match self {
//             tokio_serial::Parity::None => Parity::None,
//             tokio_serial::Parity::Odd => Parity::Odd,
//             tokio_serial::Parity::Even => Parity::Even,
//         }
//     }
// }

/// 停止位
#[derive(Clone, Debug)]
pub enum StopBits {
    /// 不使用停止位。
    None = 0,
    /// 使用一个停止位。
    One = 1,
    /// 使用两个停止位。
    Two = 2,
    /// 使用 1.5 个停止位。
    OnePointFive = 3,
}

impl From<tokio_serial::StopBits> for StopBits {
    fn from(value: tokio_serial::StopBits) -> Self {
        match value {
            tokio_serial::StopBits::One => StopBits::One,
            tokio_serial::StopBits::Two => StopBits::Two,
        }
    }
}
