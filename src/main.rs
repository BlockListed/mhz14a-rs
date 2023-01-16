use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use serial::SerialPort;
use serial::open;
use serial::PortSettings;
use pico_args::Arguments;

// UART command to send to get concentration!
const GET_CONCENTRATION_REQUEST: &[u8; 9] = &[0xff, 0x01, 0x86, 0x00, 0x00, 0x00, 0x00, 0x00, 0x79];

fn main() {
    let mut args = Arguments::from_env();

    let mut serial_path: PathBuf = PathBuf::from("/dev/ttys0");

    if let Some(path) = args.opt_value_from_str::<&str, PathBuf>("--path").unwrap() {
        serial_path = path;
    }
    let settings = PortSettings {
        baud_rate: serial::Baud9600,
        char_size: serial::Bits8,
        stop_bits: serial::Stop1,
        parity: serial::ParityNone,
        flow_control: serial::FlowNone,
    };
    let mut port = open(&serial_path).expect("Couldn't open Port");
    port.configure(&settings).expect("Couldn't configure Port.");

    assert_eq!(port.write(GET_CONCENTRATION_REQUEST).expect("Couldn't send request!"), 9);

    let mut response_buf = [0u8; 9];
    port.read_exact(&mut response_buf).expect("Couldn't receive response!");

    checksum(response_buf.as_slice()).unwrap();

    println!("{}", extract_data(response_buf.as_slice()));
}

fn checksum(data: &[u8]) -> Result<(), ()> {
    assert_eq!(data.len(), 9);
    let chksum: u8 = ( 0xff - (data[1..7].iter().sum::<u8>()) ) + 1;

    match data[8] == chksum {
        true => {
            Ok(())
        },
        false => {
            eprintln!("Checksum failed! {:#x?}", data);
            std::io::stdout().flush().unwrap();
            Err(())
        }
    } 
}

fn extract_data(data: &[u8]) -> u16 {
    assert_eq!(data.len(), 9);

    // `data[2]` are the upper 8 bits and `data[3]` are the lower 8 bits
    // The lower bits are ORed into the now empty first bits of the shifted number.
    ((data[2] as u16) << 8) | data[3] as u16
}

#[cfg(test)]
mod test {
    use crate::{checksum, extract_data};

    #[test]
    fn test_checksum() {
        let good_vectors: [&[u8; 9]; 2] = [&[0xff, 0x01, 0x86, 0x00, 0x00, 0x00, 0x00, 0x00, 0x79], &[0xff, 0x86, 0x02, 0x20, 0x00, 0x00, 0x00, 0x00, 0x58]];
        let bad_vectors: [&[u8; 9]; 2] = [&[0xff, 0x01, 0x86, 0x00, 0x00, 0x00, 0x01, 0x00, 0x79], &[0xff, 0x86, 0x02, 0x20, 0x00, 0x00, 0x00, 0x00, 0x69]];

        for i in good_vectors {
            assert!(checksum(i).is_ok());
        }
        for i in bad_vectors {
            assert!(checksum(i).is_err());
        }
    }

    #[test]
    fn test_extract_data() {
        let vector: &[u8; 9] = &[0xff, 0x86, 0x02, 0x20, 0x00, 0x00, 0x00, 0x00, 0x58];
        assert_eq!(extract_data(vector), 544);
    }
}