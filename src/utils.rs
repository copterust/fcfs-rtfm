use crate::communication::TxBuffer;

pub fn fill_with_bytes(buffer: &mut TxBuffer, arg: &[u8]) {
    buffer.extend_from_slice(arg).unwrap();
}

pub fn fill_with_str(buffer: &mut TxBuffer, arg: &str) {
    buffer.extend_from_slice(arg.as_bytes()).unwrap();
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
