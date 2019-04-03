use cortex_m_semihosting::hio;

pub existential type T: core::fmt::Write;

pub fn semihosting() -> Result<T, ()> {
    hio::hstdout()
}

// TODO: add other logging backend behind feature gates
