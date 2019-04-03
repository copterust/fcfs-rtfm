use cortex_m_semihosting::hio;

pub existential type T: core::fmt::Write;

struct Dummy;
impl core::fmt::Write for Dummy {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        Ok(())
    }

    fn write_char(&mut self, s: char) -> core::fmt::Result {
        Ok(())
    }
}

pub fn create() -> Result<T, ()> {
    #[cfg(feature = "semihosting")]
    return hio::hstdout();
    #[cfg(not(feature = "semihosting"))]
    return Ok(Dummy);
}
