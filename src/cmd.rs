use crate::types;


macro_rules! parse {
    (@cond $inp:ident $var:expr) => {
        $inp == $var.as_bytes()
    };
    (@cond $inp:ident $var:expr, $name:ident : $ty:ty) => {
        $inp.starts_with($var.as_bytes())
    };
    (@process $inp:ident $code:expr; $var:expr) => {
        $code
    };
    (@process $inp:ident $code:expr; $var:expr, $name:ident : $ty:ty) => {

        let rest = &$inp[$var.len()..];
        if let Ok($name) = utils::parse::<$ty, _>(rest) {
            $code
        }
    };
    ($input:ident:
     $([$($option:tt)+] => $code:expr),+
    ) => {
        $(
            if (parse!(@cond $input $($option)+)) {
                parse!(@process $input $code; $($option)+);
            } else
        )+
        { }
    };
}


const BUFFER_SIZE: usize = 512;
const CR: u8 = '\r' as u8;
const LF: u8 = '\n' as u8;

pub struct Cmd {
    buffer: [u8; BUFFER_SIZE],
    pos: usize,
}

impl Cmd {
    #[inline]
    pub const fn new() -> Cmd {
        Cmd { buffer: [0; BUFFER_SIZE], pos: 0 }
    }

    #[inline]
    fn push(&mut self, b: u8) -> Option<&[u8]> {
        if b == CR || b == LF {
            if self.pos == 0 {
                None
            } else {
                let result = &self.buffer[0..self.pos];
                self.pos = 0;
                Some(result)
            }
        } else {
            self.buffer[self.pos] = b;
            self.pos = (self.pos + 1) & (BUFFER_SIZE - 1);
            None
        }
    }

    #[inline]
    pub fn try_parse(&mut self, byte: u8,
                 control: &mut types::Control) {
        if let Some(word) = self.push(byte) {
            parse!(word:
                   ["tmon"] => {
                       control.enable_telemetry();
                   },
                   ["tmoff"] => {
                       control.disable_telemetry();
                   }
            );
        }
    }
}
