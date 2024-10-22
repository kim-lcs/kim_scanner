use std::{error::Error, fmt::Display};

/// 扫码枪返回致命错误
#[derive(Debug)]
pub enum ScannerError {
    /// IO 错误
    Io(std::io::Error),
    /// 参数错误(Parameter Error)
    Param(String),
    /// 通讯错误(Communicate Error)
    Comm(String),
}

impl Display for ScannerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScannerError::Io(e) => e.fmt(f),
            ScannerError::Param(e) => write!(f, "扫码枪参数错误:{}", e),
            ScannerError::Comm(e) => write!(f, "扫码枪通讯错误:{}", e),
        }
    }
}

impl Error for ScannerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
