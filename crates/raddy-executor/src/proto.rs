/// Protocol events the host observes from guest hostcalls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtoEvent {
    RespHead,
    RespWrite,
    RespEnd,
}

/// Guest ABI protocol. Illegal sequences become [`super::ExecError::Protocol`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Protocol {
    #[default]
    AwaitHead,
    Streaming,
    Ended,
}

impl Protocol {
    /// Advance on a hostcall. `Err` is the violation kind for logs.
    pub fn apply(self, event: ProtoEvent) -> Result<Self, &'static str> {
        match (self, event) {
            (Self::AwaitHead, ProtoEvent::RespHead) => Ok(Self::Streaming),
            (Self::AwaitHead, ProtoEvent::RespWrite) => Err("write before head"),
            (Self::AwaitHead, ProtoEvent::RespEnd) => Err("end before head"),
            (Self::Streaming, ProtoEvent::RespWrite) => Ok(Self::Streaming),
            (Self::Streaming, ProtoEvent::RespEnd) => Ok(Self::Ended),
            (Self::Streaming, ProtoEvent::RespHead) => Err("double resp_head"),
            (Self::Ended, _) => Err("hostcall after end"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ProtoEvent, Protocol};

    #[test]
    fn protocol_rejects_double_head_and_write_before_head() {
        let streaming = Protocol::AwaitHead
            .apply(ProtoEvent::RespHead)
            .expect("first head is legal");
        assert_eq!(
            streaming.apply(ProtoEvent::RespHead).unwrap_err(),
            "double resp_head"
        );
        assert_eq!(
            Protocol::AwaitHead
                .apply(ProtoEvent::RespWrite)
                .unwrap_err(),
            "write before head"
        );
    }
}
