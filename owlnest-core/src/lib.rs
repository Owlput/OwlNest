pub mod error {
    use std::fmt::Display;

    /// Error related to channel communication.
    #[derive(Debug)]
    pub enum ChannelError {
        /// The operation did not complete due to a timeout
        /// on a callback channel had been reached.  
        /// Future answers will be discarded.
        Timeout,
        /// The sender or receiver half of the channel has been dropped prematurely.
        Closed,
    }
    impl<T> From<tokio::sync::mpsc::error::SendError<T>> for ChannelError {
        fn from(_value: tokio::sync::mpsc::error::SendError<T>) -> Self {
            Self::Closed
        }
    }
    impl From<tokio::sync::oneshot::error::RecvError> for ChannelError {
        fn from(_value: tokio::sync::oneshot::error::RecvError) -> Self {
            Self::Closed
        }
    }
    impl std::fmt::Display for ChannelError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ChannelError::Timeout => write!(f, "Timeout waiting for response"),
                ChannelError::Closed => write!(f, "Channel closed unexpectedly"),
            }
        }
    }
    impl std::error::Error for ChannelError {}

    #[derive(Debug)]
    pub enum OperationError {
        Timeout,
        Interrupted,
    }
    impl Display for OperationError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            <Self as std::fmt::Debug>::fmt(self, f)
        }
    }
    impl std::error::Error for OperationError {}
    impl From<tokio::time::error::Elapsed> for OperationError {
        fn from(_value: tokio::time::error::Elapsed) -> Self {
            Self::Timeout
        }
    }
}

pub mod expect {
    pub const SWARM_RECEIVER_KEPT_ALIVE: &str =
        r#"Receiver in the swarm should stay alive the entire lifetime of the app."#;
    pub const CALLBACK_CLEAR: &str =
        r#"Callbacks should never be dropped without a proper cleanup."#;
    pub const GLOBAL_DEFAULT_SINGLETON: &str = r#"Global default can only be set once."#;
}

pub mod alias {
    pub type Callback<T> = tokio::sync::oneshot::Sender<T>;
}
