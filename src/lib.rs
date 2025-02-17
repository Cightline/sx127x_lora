#![allow(unused_assignments)]
#![no_std]
#![crate_type = "lib"]
#![crate_name = "sx127x_lora"]

//! # sx127x_lora
//!  A platform-agnostic driver for Semtech SX1276/77/78/79 based boards. It supports any device that
//! implements the `embedded-hal` traits. Devices are connected over SPI and require an extra GPIO pin for
//! RESET. This cate works with any Semtech based board including:
//! * Modtronix inAir4, inAir9, and inAir9B
//! * HopeRF RFM95W, RFM96W, and RFM98W
//! # Examples
//! ## Raspberry Pi Basic Send
//! Utilizes a Raspberry Pi to send a message. The example utilizes the `linux_embedded_hal` crate.
//! ```no_run
//! #![feature(extern_crate_item_prelude)]
//! extern crate sx127x_lora;
//! extern crate linux_embedded_hal as hal;
//!
//! use hal::spidev::{self, SpidevOptions};
//! use hal::{Pin, Spidev};
//! use hal::sysfs_gpio::Direction;
//! use hal::Delay;

//! const LORA_CS_PIN: u64 = 8;
//! const LORA_RESET_PIN: u64 = 21;
//! const FREQUENCY: i64 = 915;
//!
//! fn main(){
//!
//!     let mut spi = Spidev::open("/dev/spidev0.0").unwrap();
//!     let options = SpidevOptions::new()
//!         .bits_per_word(8)
//!         .max_speed_hz(20_000)
//!         .mode(spidev::SPI_MODE_0)
//!         .build();
//!     spi.configure(&options).unwrap();
//!
//!     let cs = Pin::new(LORA_CS_PIN);
//!     cs.export().unwrap();
//!     cs.set_direction(Direction::Out).unwrap();
//!
//!     let reset = Pin::new(LORA_RESET_PIN);
//!     reset.export().unwrap();
//!     reset.set_direction(Direction::Out).unwrap();
//!
//!     let mut lora = sx127x_lora::LoRa::new(
//!         spi, cs, reset,  FREQUENCY, Delay)
//!         .expect("Failed to communicate with radio module!");
//!
//!     lora.set_tx_power(17,1); //Using PA_BOOST. See your board for correct pin.
//!
//!     let message = "Hello, world!";
//!     let mut buffer = [0;255];
//!     for (i,c) in message.chars().enumerate() {
//!         buffer[i] = c as u8;
//!     }
//!
//!     let transmit = lora.transmit_payload(buffer,message.len());
//!     match transmit {
//!         Ok(packet_size) => println!("Sent packet with size: {}", packet_size),
//!         Err(()) => println!("Error"),
//!     }
//! }
//! ```
//! ## STM32F429 Blocking Receive
//! Utilizes a STM32F429 to receive data using the blocking `poll_irq(timeout)` function. It prints
//! the received packet back out over semihosting. The example utilizes the `stm32f429_hal`, `cortex_m`,
//! and `panic_semihosting` crates.
//! ```no_run
//! #![no_std]
//! #![no_main]
//!
//! extern crate sx127x_lora;
//! extern crate stm32f429_hal as hal;
//! extern crate cortex_m;
//! extern crate panic_semihosting;
//!
//! use sx127x_lora::MODE;
//! use cortex_m_semihosting::*;
//! use hal::gpio::GpioExt;
//! use hal::flash::FlashExt;
//! use hal::rcc::RccExt;
//! use hal::time::MegaHertz;
//! use hal::spi::Spi;
//! use hal::delay::Delay;
//!
//! const FREQUENCY: i64 = 915;
//!
//! #[entry]
//! fn main() -> !{
//!     let cp = cortex_m::Peripherals::take().unwrap();
//!     let p = hal::stm32f429::Peripherals::take().unwrap();
//!
//!     let mut rcc = p.RCC.constrain();
//!     let mut flash = p.FLASH.constrain();
//!     let clocks = rcc
//!         .cfgr
//!         .sysclk(MegaHertz(64))
//!         .pclk1(MegaHertz(32))
//!         .freeze(&mut flash.acr);
//!
//!     let mut gpioa = p.GPIOA.split(&mut rcc.ahb1);
//!     let mut gpiod = p.GPIOD.split(&mut rcc.ahb1);
//!     let mut gpiof = p.GPIOF.split(&mut rcc.ahb1);
//!
//!     let sck = gpioa.pa5.into_af5(&mut gpioa.moder, &mut gpioa.afrl);
//!     let miso = gpioa.pa6.into_af5(&mut gpioa.moder, &mut gpioa.afrl);
//!     let mosi = gpioa.pa7.into_af5(&mut gpioa.moder, &mut gpioa.afrl);
//!     let reset = gpiof.pf13.into_push_pull_output(&mut gpiof.moder, &mut gpiof.otyper);
//!     let cs = gpiod.pd14.into_push_pull_output(&mut gpiod.moder, &mut gpiod.otyper);
//!
//!     let spi = Spi::spi1(
//!         p.SPI1,
//!         (sck, miso, mosi),
//!         MODE,
//!         MegaHertz(8),
//!         clocks,
//!         &mut rcc.apb2,
//!     );
//!
//!     let mut lora = sx127x_lora::LoRa::new(
//!         spi, cs, reset, FREQUENCY,
//!         Delay::new(cp.SYST, clocks)).unwrap();
//!
//!     loop {
//!         let poll = lora.poll_irq(Some(30)); //30 Second timeout
//!         match poll {
//!             Ok(size) =>{
//!                hprint!("with Payload: ");
//!                let buffer = lora.read_packet(); // Received buffer. NOTE: 255 bytes are always returned
//!                for i in 0..size{
//!                    hprint!("{}",buffer[i] as char).unwrap();
//!                }
//!                hprintln!();
//!             },
//!             Err(()) => hprintln!("Timeout").unwrap(),
//!         }
//!     }
//! }
//! ```
//! ## Interrupts
//! The crate currently polls the IRQ register on the radio to determine if a new packet has arrived. This
//! would be more efficient if instead an interrupt was connect the the module's DIO_0 pin. Once interrupt
//! support is available in `embedded-hal`, then this will be added. It is possible to implement this function on a
//! device-to-device basis by retrieving a packet with the `read_packet()` function.

use bit_field::BitField;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::spi::{Transfer, Write};
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi::{Mode, Phase, Polarity};
use heapless;
use bitflags::bitflags;

pub mod register;
use self::register::*;

/// Provides the necessary SPI mode configuration for the radio
pub const MODE: Mode = Mode {
    phase: Phase::CaptureOnSecondTransition,
    polarity: Polarity::IdleHigh,
};

/*pub struct LoRaBuilder {}

impl LoRaBuilder
{
    fn new<SPI, CS, RESET>(spi: SPI, cs: CS, reset: RESET, frequency: i64, delay: Delay) -> Result<LoRaMode<SPI, CS, RESET>, ()>
    where SPI: Transfer<u8, Error = E> + Write<u8, Error = E>, CS: OutputPin, RESET: OutputPin,
    {
        Ok(LoRa::new(spi, cs, reset, frequency, delay))
    }
}*/

/// Provides high-level access to Semtech SX1276/77/78/79 based boards connected to a Raspberry Pi
pub struct LoRa<SPI, CS, RESET>
{
    spi: SPI,
    cs: CS,
    reset: RESET,
    frequency: u32,
    pub explicit_header: bool,
    pub mode: RadioMode,
}

#[derive(Debug)]
pub enum Error<SPI, CS, RESET> {
    Uninformative,
    VersionMismatch(u8),
    CS(CS),
    Reset(RESET),
    SPI(SPI),
    Transmitting,
}

pub trait Packet
{
    fn preamble(self) -> u8;
}



use Error::*;
use crate::register::{FskDataModulationShaping, FskRampUpRamDown};
use core::ops::{BitAnd, BitOr};

#[cfg(not(feature = "version_0x09"))]
const VERSION_CHECK: u8 = 0x12;

#[cfg(feature = "version_0x09")]
const VERSION_CHECK: u8 = 0x09;

impl<SPI, CS, RESET, E> LoRa<SPI, CS, RESET>
where
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E>,
    CS: OutputPin,
    RESET: OutputPin,
{
    /// Builds and returns a new instance of the radio. Only one instance of the radio should exist at a time.
    /// This also preforms a hardware reset of the module and then puts it in standby.
    pub fn new(
        spi: SPI,
        cs: CS,
        reset: RESET,
        frequency: u32,
        delay: &mut dyn DelayMs<u8>,
    ) -> Result<Self, Error<E, CS::Error, RESET::Error>> {
        let mut sx127x = LoRa {
            spi,
            cs,
            reset,
            frequency,
            explicit_header: true,
            mode: RadioMode::Sleep,
        };
        sx127x.reset.set_low().map_err(Reset)?;
        delay.delay_ms(10);
        sx127x.reset.set_high().map_err(Reset)?;
        delay.delay_ms(10);
        let version = sx127x.read_register(Register::RegVersion)?;
        if version == VERSION_CHECK {
            sx127x.set_mode(RadioMode::Sleep)?;
            sx127x.set_frequency(frequency)?;
            // Half of the FIFO is for Rx the other half for Tx. Setting both to 0 I believe allows you
            // to use the full FIFO in either Rx or Tx mode.
            sx127x.write_register(Register::RegFifoTxBaseAddr, 0)?;
            sx127x.write_register(Register::RegFifoRxBaseAddr, 0)?;
            let lna = sx127x.read_register(Register::RegLna)?;
            sx127x.write_register(Register::RegLna, lna | 0x03)?;
            sx127x.write_register(Register::RegModemConfig3, 0x04)?;
            sx127x.set_mode(RadioMode::Stdby)?;
            sx127x.cs.set_high().map_err(CS)?;
            Ok(sx127x)
        } else {
            Err(Error::VersionMismatch(version))
        }
    }

    /// Transmits up to 255 bytes of data. To avoid the use of an allocator, this takes a fixed 255 u8
    /// array and a payload size and returns the number of bytes sent if successful.
    /*pub fn transmit_payload_busy(
        &mut self,
        buffer: [u8; 255],
        payload_size: usize,
    ) -> Result<usize, Error<E, CS::Error, RESET::Error>> {
        if self.transmitting()? {
            Err(Transmitting)
        } else {
            self.set_mode(RadioMode::Stdby)?;
            if self.explicit_header {
                self.set_explicit_header_mode()?;
            } else {
                self.set_implicit_header_mode()?;
            }

            self.write_register(Register::RegIrqFlags, 0)?;
            self.write_register(Register::RegFifoAddrPtr, 0)?;
            self.write_register(Register::RegPayloadLength, 0)?;
            for byte in buffer.iter().take(payload_size) {
                self.write_register(Register::RegFifo, *byte)?;
            }
            self.write_register(Register::RegPayloadLength, payload_size as u8)?;
            self.set_mode(RadioMode::Tx)?;
            while self.transmitting()? {}
            Ok(payload_size)
        }
    }*/

    pub fn set_dio0_tx_done(&mut self) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        self.write_register(Register::RegDioMapping1, 0b01_00_00_00)
    }

    /*pub fn transmit_packet(&mut self, packet: Packet) -> Result<(), Error<E, CS::Error, RESET::Error>>
    {
        Ok(())
    }*/

    //pub fn transmit_payload(&mut self, buffer: [u8; 255], payload_size: usize) -> Result<(), Error<E, CS::Error, RESET::Error>>
    pub fn transmit_payload(&mut self, payload: &heapless::Vec<u8, 255>) -> Result<(), Error<E, CS::Error, RESET::Error>>
    {
        // Variable length packet (page 73):
        // Variable length packet format is selected when bit PacketFormat is set to 1.
        // In this mode the length of the payload, indicated by the length byte, is given by the first byte of the FIFO and is limited to 255 bytes.
        // In this mode, the payload must contain at least 2 bytes, i.e. length + address or message byte

        /*if self.transmitting()?
        {
            return Err(Transmitting);
        }*/

        self.set_mode(RadioMode::Stdby)?;

        if self.explicit_header
        {
            self.set_explicit_header_mode()?;
        }

        else
        {
            self.set_implicit_header_mode()?;
        }

        self.write_register(Register::RegIrqFlags, 0)?;
        self.write_register(Register::RegFifoAddrPtr, 0)?;
        self.write_register(Register::RegPayloadLength, 0)?;

        let length_byte = payload.len() as u8;

        self.write_register(Register::RegFifo, length_byte)?;

        for byte in payload.iter()
        {
            self.write_register(Register::RegFifo, *byte)?;
        }

        //self.write_register(Register::RegPayloadLength, payload_size as u8)?;

        self.set_mode(RadioMode::Tx)
    }

    /// Blocks the current thread, returning the size of a packet if one is received or an error is the
    /// task timed out. The timeout can be supplied with None to make it poll indefinitely or
    /// with `Some(timeout_in_mill_seconds)`
    pub fn poll_irq(
        &mut self,
        timeout_ms: Option<i32>,
        delay: &mut dyn DelayMs<u8>,
    ) -> Result<usize, Error<E, CS::Error, RESET::Error>> {
        self.set_mode(RadioMode::RxContinuous)?;
        match timeout_ms {
            Some(value) => {
                let mut count = 0;
                let packet_ready = loop {
                    let packet_ready = self.read_register(Register::RegIrqFlags)?.get_bit(6);
                    if count >= value || packet_ready {
                        break packet_ready;
                    }
                    count += 1;
                    delay.delay_ms(1);
                };
                if packet_ready {
                    self.clear_irq()?;
                    Ok(self.read_register(Register::RegRxNbBytes)? as usize)
                } else {
                    Err(Uninformative)
                }
            }
            None => {
                while !self.read_register(Register::RegIrqFlags)?.get_bit(6) {
                    delay.delay_ms(100);
                }
                self.clear_irq()?;
                Ok(self.read_register(Register::RegRxNbBytes)? as usize)
            }
        }
    }

    pub fn is_packet_ready(&mut self) -> Result<bool, Error<E, CS::Error, RESET::Error>> {
        Ok(self.read_register(Register::RegIrqFlags)? & 0x04 != 0)
    }

    /// Returns the contents of the fifo as a fixed 255 u8 array. This should only be called is there is a
    /// new packet ready to be read.
    pub fn read_packet(&mut self) -> Result<[u8; 255], Error<E, CS::Error, RESET::Error>> {
        let mut buffer = [0 as u8; 255];
        self.clear_irq()?;
        let size = self.read_register(Register::RegRxNbBytes)?;
        let fifo_addr = self.read_register(Register::RegFifoRxCurrentAddr)?;
        self.write_register(Register::RegFifoAddrPtr, fifo_addr)?;
        for i in 0..size {
            let byte = self.read_register(Register::RegFifo)?;
            buffer[i as usize] = byte;
        }
        self.write_register(Register::RegFifoAddrPtr, 0)?;
        Ok(buffer)
    }

    /*pub fn is_fifo_full(&mut self) -> Result<u8, Error<E, CS::Error, RESET::Error>>
    {

    }

    pub fn is_fifo_threshold(&mut self) -> Result<u8, Error<E, CS::Error, RESET::Error>>
    {
        self.read_register(Register::RegIrqFlags)? & IRQ::IrqTxDoneMask == 1
    }*/

    pub fn irq_flags(&mut self) -> Result<u8, Error<E, CS::Error, RESET::Error>>
    {
        self.read_register(Register::RegIrqFlags)
    }

    /*/// Returns true if the radio is currently transmitting a packet.
    pub fn transmitting(&mut self) -> Result<bool, Error<E, CS::Error, RESET::Error>> {
        if (self.read_register(Register::RegOpMode)? & RadioMode::Tx as u8) == RadioMode::Tx
        {
            Ok(true)
        }

        else
        {
            if (self.read_register(Register::RegIrqFlags)? & IrqMask::TxDone) == 1
            {
                self.write_register(Register::RegIrqFlags, IrqMask::TxDone)?;
            }
            Ok(false)
        }
    }*/

    /// Clears the radio's IRQ registers.
    pub fn clear_irq(&mut self) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        let irq_flags = self.read_register(Register::RegIrqFlags)?;
        self.write_register(Register::RegIrqFlags, irq_flags)
    }

    /// Sets the transmit power and pin. Levels can range from 0-14 when the output
    /// pin = 0(RFO), and from 0-20 when output pin = 1(PaBoost). Power is in dB.
    /// Default value is `17`.
    /// https://github.com/PaulStoffregen/RadioHead/blob/master/RH_RF95.cpp#L435
    /// https://cdn-shop.adafruit.com/product-files/3179/sx1276_77_78_79.pdf
    pub fn set_tx_power(&mut self, mut level: u8, use_rfo: bool) -> Result<(), Error<E, CS::Error, RESET::Error>>
    {
        // TODO: fix

        Ok(())


        /* I have no idea as to what this is doing.
        if PaConfig::PaOutputRfoPin == output_pin
        {
            if level > 14
            {
                level = 14;
            }
            self.write_register(Register::RegPaConfig, (0x70 | level))
        }

        else
        {
            // PA BOOST
            if level > 17
            {
                if level > 20 {
                    level = 20;
                }
                // subtract 3 from level, so 18 - 20 maps to 15 - 17
                level -= 3;

                // High Power +20 dBm Operation (Semtech SX1276/77/78/79 5.4.3.)
                self.write_register(Register::RegPaDac, 0x87)?;
                self.set_ocp(140)?;
            } else {
                if level < 2 {
                    level = 2;
                }
                //Default value PA_HF/LF or +17dBm
                self.write_register(Register::RegPaDac, 0x84)?;
                self.set_ocp(100)?;
            }
            level -= 2;
            self.write_register(
                Register::RegPaConfig,
                PaConfig::PaBoost | level as u8,
            )
        }*/
    }

    /// Sets the over current protection on the radio(mA).
    pub fn set_ocp(&mut self, ma: u8) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        let mut ocp_trim: u8 = 27;

        if ma <= 120 {
            ocp_trim = (ma - 45) / 5;
        } else if ma <= 240 {
            ocp_trim = (ma + 30) / 10;
        }
        self.write_register(Register::RegOcp, 0x20 | (0x1F & ocp_trim))
    }

    /// Sets the state of the radio. Default mode after initiation is `Standby`.
    pub fn set_mode(&mut self, mode: RadioMode) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        if self.explicit_header {
            self.set_explicit_header_mode()?;
        } else {
            self.set_implicit_header_mode()?;
        }
        self.write_register(Register::RegOpMode, RadioMode::LongRangeMode as u8 | mode as u8)?;

        self.mode = mode;
        Ok(())
    }

    /// Sets the frequency of the radio. Values are in megahertz.
    /// I.E. 915 MHz must be used for North America. Check regulation for your area.
    pub fn set_frequency(&mut self, freq: u32) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        self.frequency = freq;
        // calculate register values
        let base = 1;
        let frf = (freq * (base << 19)) / 32;
        // write registers
        self.write_register(
            Register::RegFrfMsb,
            ((frf & 0x00FF_0000) >> 16) as u8,
        )?;
        self.write_register(Register::RegFrfMid, ((frf & 0x0000_FF00) >> 8) as u8)?;
        self.write_register(Register::RegFrfLsb, (frf & 0x0000_00FF) as u8)
    }

    /// Sets the radio to use an explicit header. Default state is `ON`.
    fn set_explicit_header_mode(&mut self) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        let reg_modem_config_1 = self.read_register(Register::RegModemConfig1)?;
        self.write_register(Register::RegModemConfig1, reg_modem_config_1 & 0xfe)?;
        self.explicit_header = true;
        Ok(())
    }

    /// Sets the radio to use an implicit header. Default state is `OFF`.
    fn set_implicit_header_mode(&mut self) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        let reg_modem_config_1 = self.read_register(Register::RegModemConfig1)?;
        self.write_register(Register::RegModemConfig1, reg_modem_config_1 & 0x01)?;
        self.explicit_header = false;
        Ok(())
    }

    /// Sets the spreading factor of the radio. Supported values are between 6 and 12.
    /// If a spreading factor of 6 is set, implicit header mode must be used to transmit
    /// and receive packets. Default value is `7`.
    pub fn set_spreading_factor(
        &mut self,
        mut sf: u8,
    ) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        if sf < 6 {
            sf = 6;
        } else if sf > 12 {
            sf = 12;
        }

        if sf == 6 {
            self.write_register(Register::RegDetectionOptimize, 0xc5)?;
            self.write_register(Register::RegDetectionThreshold, 0x0c)?;
        } else {
            self.write_register(Register::RegDetectionOptimize, 0xc3)?;
            self.write_register(Register::RegDetectionThreshold, 0x0a)?;
        }
        let modem_config_2 = self.read_register(Register::RegModemConfig2)?;
        self.write_register(
            Register::RegModemConfig2,
            (modem_config_2 & 0x0f) | ((sf << 4) & 0xf0),
        )?;
        self.set_ldo_flag()?;
        Ok(())
    }

    /// Sets the signal bandwidth of the radio. Supported values are: `7800 Hz`, `10400 Hz`,
    /// `15600 Hz`, `20800 Hz`, `31250 Hz`,`41700 Hz` ,`62500 Hz`,`125000 Hz` and `250000 Hz`
    /// Default value is `125000 Hz`
    pub fn set_signal_bandwidth(
        &mut self,
        sbw: i64,
    ) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        let bw: i64 = match sbw {
            7_800 => 0,
            10_400 => 1,
            15_600 => 2,
            20_800 => 3,
            31_250 => 4,
            41_700 => 5,
            62_500 => 6,
            125_000 => 7,
            250_000 => 8,
            _ => 9,
        };
        let modem_config_1 = self.read_register(Register::RegModemConfig1)?;
        self.write_register(
            Register::RegModemConfig1,
            (modem_config_1 & 0x0f) | ((bw << 4) as u8),
        )?;
        self.set_ldo_flag()?;
        Ok(())
    }

    /// Sets the coding rate of the radio with the numerator fixed at 4. Supported values
    /// are between `5` and `8`, these correspond to coding rates of `4/5` and `4/8`.
    /// Default value is `5`.
    pub fn set_coding_rate_4(
        &mut self,
        mut denominator: u8,
    ) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        if denominator < 5 {
            denominator = 5;
        } else if denominator > 8 {
            denominator = 8;
        }
        let cr = denominator - 4;
        let modem_config_1 = self.read_register(Register::RegModemConfig1)?;
        self.write_register(
            Register::RegModemConfig1,
            (modem_config_1 & 0xf1) | (cr << 1),
        )
    }

    /// Sets the preamble length of the radio. Values are between 6 and 65535.
    /// Default value is `8`.
    pub fn set_preamble_length(
        &mut self,
        length: i64,
    ) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        self.write_register(Register::RegPreambleMsb, (length >> 8) as u8)?;
        self.write_register(Register::RegPreambleLsb, length as u8)
    }

    /// Enables are disables the radio's CRC check. Default value is `false`.
    pub fn set_crc(&mut self, value: bool) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        let modem_config_2 = self.read_register(Register::RegModemConfig2)?;
        if value {
            self.write_register(Register::RegModemConfig2, modem_config_2 | 0x04)
        } else {
            self.write_register(Register::RegModemConfig2, modem_config_2 & 0xfb)
        }
    }

    /// Inverts the radio's IQ signals. Default value is `false`.
    pub fn set_invert_iq(&mut self, value: bool) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        if value {
            self.write_register(Register::RegInvertiq, 0x66)?;
            self.write_register(Register::RegInvertiq2, 0x19)
        } else {
            self.write_register(Register::RegInvertiq, 0x27)?;
            self.write_register(Register::RegInvertiq2, 0x1d)
        }
    }

    /// Returns the spreading factor of the radio.
    pub fn get_spreading_factor(&mut self) -> Result<u8, Error<E, CS::Error, RESET::Error>> {
        Ok(self.read_register(Register::RegModemConfig2)? >> 4)
    }

    /// Returns the signal bandwidth of the radio.
    pub fn get_signal_bandwidth(&mut self) -> Result<i64, Error<E, CS::Error, RESET::Error>> {
        let bw = self.read_register(Register::RegModemConfig1)? >> 4;
        let bw = match bw {
            0 => 7_800,
            1 => 10_400,
            2 => 15_600,
            3 => 20_800,
            4 => 31_250,
            5 => 41_700,
            6 => 62_500,
            7 => 125_000,
            8 => 250_000,
            9 => 500_000,
            _ => -1,
        };
        Ok(bw)
    }

    /// Returns the RSSI of the last received packet.
    pub fn get_packet_rssi(&mut self) -> Result<i32, Error<E, CS::Error, RESET::Error>> {
        Ok(i32::from(self.read_register(Register::RegPktRssiValue)?) - 157)
    }

    /// Returns the signal to noise radio of the the last received packet.
    pub fn get_packet_snr(&mut self) -> Result<f64, Error<E, CS::Error, RESET::Error>> {
        Ok(f64::from(
            self.read_register(Register::RegPktSnrValue)?,
        ))
    }

    /// Returns the frequency error of the last received packet in Hz.
    pub fn get_packet_frequency_error(&mut self) -> Result<i64, Error<E, CS::Error, RESET::Error>> {
        let mut freq_error: i32 = 0;
        freq_error = i32::from(self.read_register(Register::RegFreqErrorMsb)? & 0x7);
        freq_error <<= 8i64;
        freq_error += i32::from(self.read_register(Register::RegFreqErrorMid)?);
        freq_error <<= 8i64;
        freq_error += i32::from(self.read_register(Register::RegFreqErrorLsb)?);

        let f_xtal = 32_000_000; // FXOSC: crystal oscillator (XTAL) frequency (2.5. Chip Specification, p. 14)
        let f_error = ((f64::from(freq_error) * (1i64 << 24) as f64) / f64::from(f_xtal))
            * (self.get_signal_bandwidth()? as f64 / 500_000.0f64); // p. 37
        Ok(f_error as i64)
    }

    fn set_ldo_flag(&mut self) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        let sw = self.get_signal_bandwidth()?;
        // Section 4.1.1.5
        let symbol_duration = 1000 / (sw / ((1 as i64) << self.get_spreading_factor()?));

        // Section 4.1.1.6
        let ldo_on = symbol_duration > 16;

        let mut config_3 = self.read_register(Register::RegModemConfig3)?;
        config_3.set_bit(3, ldo_on);
        self.write_register(Register::RegModemConfig3, config_3)
    }

    pub fn read_register(&mut self, reg: Register) -> Result<u8, Error<E, CS::Error, RESET::Error>> {
        self.cs.set_low().map_err(CS)?;

        let mut buffer = [reg as u8 & 0x7f, 0];
        let transfer = self.spi.transfer(&mut buffer).map_err(SPI)?;
        self.cs.set_high().map_err(CS)?;
        Ok(transfer[1])
    }

    fn write_register(
        &mut self,
        reg: Register,
        byte: u8,
    ) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        self.cs.set_low().map_err(CS)?;

        let buffer = [reg as u8 | 0x80, byte];
        self.spi.write(&buffer).map_err(SPI)?;
        self.cs.set_high().map_err(CS)?;
        Ok(())
    }

    /*pub fn put_in_fsk_mode(&mut self) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        // Put in FSK mode
        let op_mode: &mut u8 = 0x0
            .set_bit(7, false)  // FSK mode
            .set_bits(5..6, 0x00)   // FSK modulation
            .set_bit(3, false)  //Low freq registers
            .set_bits(0..2, 0b011); // Mode

        self.write_register(Register::RegOpMode as u8, *op_mode)
    }*/

    /*pub fn set_fsk_pa_ramp(
        &mut self,
        modulation_shaping: FskDataModulationShaping,
        ramp: FskRampUpRamDown
    ) -> Result<(), Error<E, CS::Error, RESET::Error>> {
        let pa_ramp: &mut u8 = 0x0
            .set_bits(5..6, modulation_shaping as u8)
            .set_bits(0..3, ramp as u8);

        self.write_register(Register::RegPaRamp as u8, *pa_ramp)
    }*/
}
/// Modes of the radio and their corresponding register values.
#[derive(Clone, Copy)]
pub enum RadioMode {
    LongRangeMode = 0x80,
    Sleep = 0x00,
    Stdby = 0x01,
    Tx = 0x03,
    RxContinuous = 0x05,
    RxSingle = 0x06,
}


bitflags! {
    struct Flags: u32 {
        const A = 0b00000001;
        const B = 0b00000010;
        const C = 0b00000100;
        const ABC = Self::A.bits | Self::B.bits | Self::C.bits;
    }
}



/*impl BitAnd<register::IrqMask> for u8
{
    type Output = Self;

    fn bitand(self, h: u8) -> <Self as BitAnd<u8>>::Output
    {
        self as u8 & h
    }
}

impl BitAnd<u8> for RadioMode
{
    type Output = Self;

    fn bitand(self, h: u8) -> <Self as BitAnd<u8>>::Output
    {
        self & h as u8
    }
}

impl BitOr<RadioMode> for RadioMode
{

    type Output = Self;

    fn bitor(self, h: u8) -> <Self as BitAnd<u8>>::Output
    {
        self as u8 | h
    }
}*/
