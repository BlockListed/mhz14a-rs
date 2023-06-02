use std::io::Read;
use std::io::Write;
use std::num::Wrapping;
use std::path::PathBuf;

use log::error;

use log::warn;
use pico_args::Arguments;
use serial::open;
use serial::PortSettings;
use serial::SerialPort;

mod statements;

// UART command to send to get concentration!
const GET_CONCENTRATION_REQUEST: [u8; 9] = [0xff, 0x01, 0x86, 0x00, 0x00, 0x00, 0x00, 0x00, 0x79];

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let mut args = Arguments::from_env();

    if args.contains("--license") {
        println!("{}", statements::LICENSE);
        return;
    }

    if args.contains("--version") {
        println!("{}", statements::VERSION);
        return;
    }

    if args.contains("--help") {
        println!("{}", statements::HELP);
        return;
    }

    let mut serial_path: PathBuf = PathBuf::from("/dev/ttyS0");

    if let Some(path) = args.opt_value_from_str::<_, PathBuf>("--path").unwrap() {
        serial_path = path;
    }
    let ignore_checksum = args.contains("--ignore-checksum");
    let settings = PortSettings {
        baud_rate: serial::Baud9600,
        char_size: serial::Bits8,
        stop_bits: serial::Stop1,
        parity: serial::ParityNone,
        flow_control: serial::FlowNone,
    };
    let mut port = open(&serial_path).expect("Couldn't open Port");
    port.configure(&settings).expect("Couldn't configure Port.");

    port.write_all(&GET_CONCENTRATION_REQUEST)
        .expect("Couldn't send request!");

    let response_buf = &mut [0u8; 9];
    port.read_exact(response_buf)
        .expect("Couldn't receive response!");

    if let Err(e) = verify_checksum(response_buf) {
        if ignore_checksum {
            warn!("Ignored invalid checksum: {:#x}!", e);
        } else {
            error!("Invalid checksum: {:#x}!", e);
            return;
        }
    }

    println!("{}", extract_data(response_buf));
}

// Python: ((0xff - (sum(data[1:7]) % (1<<8))) + 1) % (1<<8))
// Modulo 1<<8 since python isn't using an 8bit wide type capable of overflowing.
fn checksum(data: &[u8; 9]) -> u8 {
    let chksum: u8 = ((Wrapping(0xff)
        - (data[1..7]
            .iter()
            .copied()
            .map(Wrapping)
            // Sum values together
            .reduce(|acc, x| acc + x)
            .unwrap()))
        + Wrapping(1))
    .0;

    chksum
}

fn verify_checksum(data: &[u8; 9]) -> Result<u8, u8> {
    let expected_chk = extract_checksum(data);
    let chk = checksum(data);

    if expected_chk == chk {
        Ok(chk)
    } else {
        Err(chk)
    }
}

fn extract_checksum(data: &[u8; 9]) -> u8 {
    // Checksum is always the last byte.
    *data.last().unwrap()
}

fn extract_data(data: &[u8; 9]) -> u16 {
    // `data[2]` are the upper 8 bits and `data[3]` are the lower 8 bits
    // The lower bits are ORed into the now empty first bits of the shifted number.
    ((u16::from(data[2])) << 8) | u16::from(data[3])
}

#[cfg(test)]
mod test {
    use crate::{extract_data, verify_checksum, GET_CONCENTRATION_REQUEST};

    #[test]
    fn test_request_payload() {
        assert!(verify_checksum(&GET_CONCENTRATION_REQUEST).is_ok())
    }

    #[test]
    fn test_checksum() {
        // First vector is from computer to sensor.
        // Second vector is from sensor to computer.
        // Third vector is from sensor to computer and is designed to check if the sum correctly overflows.
        // Fourth vector is from sensor to computer and is designed to check if the final +1 correctly overflows.
        let good_vectors: [&[u8; 9]; 4] = [
            &[0xff, 0x01, 0x86, 0x00, 0x00, 0x00, 0x00, 0x00, 0x79],
            &[0xff, 0x86, 0x02, 0x20, 0x00, 0x00, 0x00, 0x00, 0x58],
            &[0xff, 0x86, 0x01, 0xfe, 0x00, 0x00, 0x00, 0x00, 0x7b],
            &[0xff, 0x86, 0x01, 0x79, 0x00, 0x00, 0x00, 0x00, 0x00],
        ];
        let bad_vectors: [&[u8; 9]; 4] = [
            &[0xff, 0x01, 0x86, 0x00, 0x00, 0x00, 0x01, 0x00, 0x79],
            &[0xff, 0x86, 0x02, 0x20, 0x00, 0x00, 0x00, 0x00, 0x69],
            &[0xff, 0x86, 0x01, 0xfd, 0x00, 0x00, 0x00, 0x00, 0x7b],
            &[0xff, 0x86, 0x01, 0x78, 0x00, 0x00, 0x00, 0x00, 0x00],
        ];

        for i in good_vectors {
            if let Err(e) = verify_checksum(i) {
                panic!("Should be GOOD: {:#?}", e);
            }
        }
        for i in bad_vectors {
            if let Ok(e) = verify_checksum(i) {
                panic!("Should be BAD!: {:#?}", e);
            }
        }
    }

    #[test]
    fn test_extract_data() {
        let vector: &[u8; 9] = &[0xff, 0x86, 0x02, 0x20, 0x00, 0x00, 0x00, 0x00, 0x58];
        assert_eq!(extract_data(vector), 544);
    }
}
