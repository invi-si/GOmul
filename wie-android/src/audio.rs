//! Convert existing decoded WIE audio to Android-supported MIDI/WAV containers.
use wie_backend::{AudioCommand, AudioEventData};
fn vlq(mut n: u64, out: &mut Vec<u8>) {
    let mut b = [0u8; 10];
    let mut p = 9;
    b[p] = (n & 127) as u8;
    n >>= 7;
    while n != 0 {
        p -= 1;
        b[p] = ((n & 127) as u8) | 128;
        n >>= 7;
    }
    out.extend_from_slice(&b[p..]);
}
fn part(out: &mut Vec<u8>, delay: u64, kind: u8, data: &[u8]) {
    out.extend_from_slice(&delay.to_le_bytes());
    out.push(kind);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
}
pub fn packet(command: AudioCommand) -> Vec<u8> {
    let mut out = Vec::new();
    match command {
        AudioCommand::Stop { handle } => {
            out.push(0);
            out.extend_from_slice(&handle.to_le_bytes());
        }
        AudioCommand::Play { handle, sequence, repeat } => {
            out.push(1);
            out.extend_from_slice(&handle.to_le_bytes());
            out.extend_from_slice(&sequence.duration.to_le_bytes());
            out.push(u8::from(repeat));
            let mut track = vec![0, 0xff, 0x51, 3, 0x07, 0xa1, 0x20]; // 500000us/qn, 500 ticks/qn => 1ms/tick
            let mut last = 0;
            let mut parts = Vec::new();
            let mut count = 0u32;
            let mut midi = false;
            for e in &sequence.events {
                match &e.data {
                    AudioEventData::Midi(bytes) => {
                        if bytes.is_empty() {
                            continue;
                        }
                        let status = bytes[0];
                        if (0x80..0xf0).contains(&status) {
                            vlq(e.time.saturating_sub(last), &mut track);
                            track.extend_from_slice(bytes);
                            last = e.time;
                            midi = true;
                        } else if status == 0xf0 || status == 0xf7 {
                            vlq(e.time.saturating_sub(last), &mut track);
                            track.push(status);
                            vlq((bytes.len() - 1) as u64, &mut track);
                            track.extend_from_slice(&bytes[1..]);
                            last = e.time;
                            midi = true;
                        }
                    }
                    AudioEventData::Wave {
                        channels,
                        sampling_rate,
                        samples,
                    } => {
                        let size = (samples.len() * 2) as u32;
                        let mut wav = b"RIFF".to_vec();
                        wav.extend_from_slice(&(36 + size).to_le_bytes());
                        wav.extend_from_slice(b"WAVEfmt ");
                        wav.extend_from_slice(&16u32.to_le_bytes());
                        wav.extend_from_slice(&1u16.to_le_bytes());
                        wav.extend_from_slice(&u16::from(*channels).to_le_bytes());
                        wav.extend_from_slice(&sampling_rate.to_le_bytes());
                        wav.extend_from_slice(&(sampling_rate * u32::from(*channels) * 2).to_le_bytes());
                        wav.extend_from_slice(&(u16::from(*channels) * 2).to_le_bytes());
                        wav.extend_from_slice(&16u16.to_le_bytes());
                        wav.extend_from_slice(b"data");
                        wav.extend_from_slice(&size.to_le_bytes());
                        for x in samples {
                            wav.extend_from_slice(&x.to_le_bytes());
                        }
                        part(&mut parts, e.time, 1, &wav);
                        count += 1;
                    }
                }
            }
            if midi {
                vlq(sequence.duration.saturating_sub(last), &mut track);
                track.extend_from_slice(&[0xff, 0x2f, 0]);
                let mut smf = b"MThd\0\0\0\x06\0\0\0\x01\x01\xf4MTrk".to_vec();
                smf.extend_from_slice(&(track.len() as u32).to_be_bytes());
                smf.extend_from_slice(&track);
                part(&mut parts, 0, 0, &smf);
                count += 1;
            }
            out.extend_from_slice(&count.to_le_bytes());
            out.extend(parts);
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wie_backend::{AudioSequence, TimedAudioEvent};
    #[test]
    fn midi_preserves_millisecond_timing_and_end_duration() {
        let p = packet(AudioCommand::Play {
            handle: 7,
            repeat: true,
            sequence: Arc::new(AudioSequence {
                duration: 1000,
                events: vec![
                    TimedAudioEvent {
                        time: 200,
                        data: AudioEventData::Midi(vec![0x90, 60, 100]),
                    },
                    TimedAudioEvent {
                        time: 500,
                        data: AudioEventData::Midi(vec![0x80, 60, 0]),
                    },
                ],
            }),
        });
        assert_eq!(&p[1..5], &7u32.to_le_bytes());
        assert_eq!(p[13], 1);
        assert!(p.windows(4).any(|x| x == b"MThd"));
        assert!(p.windows(5).any(|x| x == [0x81, 0x48, 0x90, 60, 100]));
        assert!(p.ends_with(&[0x83, 0x74, 0xff, 0x2f, 0]));
    }
}
