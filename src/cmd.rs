use crate::types;

fn parse<T, E>(bytes: &[u8]) -> Result<T, E>
    where T: core::str::FromStr<Err = E>
{
    let v = unsafe { core::str::from_utf8_unchecked(bytes) };
    T::from_str(v)
}

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
        if let Ok($name) = parse::<$ty, _>(rest) {
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

pub const fn create() -> Cmd {
    Cmd::new()
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
    pub fn feed(&mut self, byte: u8, control: &mut types::Control) -> Option<types::Requests> {
        let mut requests = None;
        if let Some(word) = self.push(byte) {
            // XXX: maybe return new control, instead of mutating?
            parse!(word:
                   ["tmon"] => {
                       control.telemetry = true;
                   },
                   ["tmoff"] => {
                       control.telemetry = false;
                   },
                   ["pk=", pk:i32] => {
                       control.pk = pk as f32;
                   },
                   ["ik=", ik:i32] => {
                       control.ik = ik as f32;
                   },
                   ["dk=", dk:i32] => {
                       control.dk = dk as f32;
                   },
                   ["pipk=", pitch_pk:i32] => {
                       control.pitch_pk = pitch_pk as f32;
                   },
                   ["rpk=", roll_pk:i32] => {
                       control.roll_pk = roll_pk as f32;
                   },
                   ["ypk=", yaw_pk:i32] => {
                       control.yaw_pk = yaw_pk as f32;
                   },
                   ["tthurst=", thrust:i32] => {
                       control.thrust = thrust as f32;
                   },
                   ["pt=", pt:i32] => {
                       control.target_degrees.pitch = pt as f32;
                   },
                   ["status"] => {
                       requests = Some(types::Requests::Status);
                   },
                   ["boot"] => {
                       requests = Some(types::Requests::Boot);
                   },
                   ["reset"] => {
                       requests = Some(types::Requests::Reset);
                   }
            );
        }

        requests
    }
}
