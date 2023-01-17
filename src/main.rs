use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use pico_args::Arguments;
use serial::open;
use serial::PortSettings;
use serial::SerialPort;

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

    assert_eq!(
        port.write(GET_CONCENTRATION_REQUEST)
            .expect("Couldn't send request!"),
        9
    );

    let response_buf = &mut [0u8; 9];
    port.read_exact(response_buf)
        .expect("Couldn't receive response!");

    checksum(response_buf).unwrap();

    println!("{}", extract_data(response_buf));
}

// Python: (0xff - ((b1 + b2 + b3 + b4 + b5 + b6 + b7) % (1<<8))) + 1
// Module 1<<8 since python isn't using an 8bit wide type capable of overflowing.
fn checksum<'a>(data: &[u8; 9]) -> Result<u8, u8> {
    assert_eq!(data.len(), 9);
    let chksum: u8 = (0xff
        - (data[1..7]
            .iter()
            .map(|x| *x)
            .reduce(|acc, x| acc.overflowing_add(x).0)
            .unwrap()))
        + 1;

    match data[8] == chksum {
        true => Ok(chksum),
        false => {
            eprintln!("Checksum failed! {:#x?}", data);
            // Conditionally compiled, since this is only needs for tests,
            // because they are run in parallel and otherwise it fucks up
            // the output.
            #[cfg(test)]
            std::io::stdout().flush().unwrap();
            Err(chksum)
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
    use crate::{checksum, extract_data, GET_CONCENTRATION_REQUEST};

    #[test]
    fn test_request_payload() {
        assert!(checksum(GET_CONCENTRATION_REQUEST).is_ok())
    }

    #[test]
    fn test_checksum() {
        // First vector is command sent from computer to sensor.
        // Second vector is command sent from sensor to computer.
        // Third command is from sensor to computer and designed to check what happens in the event of an overflow.
        let good_vectors: [&[u8; 9]; 3] = [
            &[0xff, 0x01, 0x86, 0x00, 0x00, 0x00, 0x00, 0x00, 0x79],
            &[0xff, 0x86, 0x02, 0x20, 0x00, 0x00, 0x00, 0x00, 0x58],
            &[0xff, 0x86, 0x01, 0xfe, 0x00, 0x00, 0x00, 0x00, 0x7b],
        ];
        let bad_vectors: [&[u8; 9]; 3] = [
            &[0xff, 0x01, 0x86, 0x00, 0x00, 0x00, 0x01, 0x00, 0x79],
            &[0xff, 0x86, 0x02, 0x20, 0x00, 0x00, 0x00, 0x00, 0x69],
            &[0xff, 0x86, 0x01, 0xfd, 0x00, 0x00, 0x00, 0x00, 0x7b],
        ];

        for i in good_vectors {
            match checksum(i) {
                Ok(_) => (),
                Err(x) => {
                    panic!("Should be GOOD {}", x);
                }
            }
        }
        for i in bad_vectors {
            match checksum(i) {
                Ok(x) => {
                    panic!("Should be BAD! {}", x);
                }
                Err(_) => (),
            }
        }
    }

    #[test]
    fn test_extract_data() {
        let vector: &[u8; 9] = &[0xff, 0x86, 0x02, 0x20, 0x00, 0x00, 0x00, 0x00, 0x58];
        assert_eq!(extract_data(vector), 544);
    }
}
