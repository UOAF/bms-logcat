use std::{collections::BTreeSet, io::prelude::*};

use anyhow::{anyhow, ensure, Result};
use byte_struct::*;
use byteorder::{ReadBytesExt, WriteBytesExt, LE};
use camino::Utf8PathBuf;
use enum_iterator::IntoEnumIterator;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, IntoPrimitive, TryFromPrimitive, Serialize, Deserialize)]
#[repr(i32)]
pub enum Rank {
    SecondLt,
    Leiutenant,
    Captain,
    Major,
    LtColonel,
    Colonel,
    BrigadierGeneral,
}

impl Default for Rank {
    fn default() -> Self {
        Rank::SecondLt
    }
}

#[derive(
    Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, IntoEnumIterator, Serialize, Deserialize,
)]
pub enum Medals {
    AirForceCross,
    SilverStar,
    DistinguishedFlyingCross,
    AirMedal,
    KoreaCampaign,
    Longevity,
}

#[derive(Debug, Default, ByteStruct, Serialize, Deserialize)]
#[byte_struct_le]
pub struct DogfightStats {
    pub matches_won: i16,
    pub matches_lost: i16,
    pub matches_won_versus_humans: i16,
    pub matches_lost_versus_humans: i16,
    pub kills: i16,
    pub killed: i16,
    pub human_kills: i16,
    pub killed_versus_humans: i16,
}

#[derive(Debug, Default, ByteStruct, Serialize, Deserialize)]
#[byte_struct_le]
pub struct CampaignStats {
    pub games_won: i16,
    pub game_lost: i16,
    pub games_tied: i16,
    pub missions: i16,
    pub total_score: i32,
    pub total_mission_score: i32,
    pub consecutive_missions: i16,
    pub kills: i16,
    pub killed: i16,
    pub human_kills: i16,
    pub killed_versus_humans: i16,
    pub self_kills: i16,
    pub air_to_ground_kills: i16,
    pub static_kills: i16,
    pub naval_kills: i16,
    pub friendly_kills: i16,
    pub missions_since_last_friendly_kill: i16,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Logbook {
    pub name: String,
    pub callsign: String,
    pub password: String,
    pub commissioned: String,
    pub options_file: Utf8PathBuf,
    pub flight_hours: f32,
    pub ace_factor: f32,
    pub rank: Rank,
    pub dogfight_stats: DogfightStats,
    pub campaign_stats: CampaignStats,
    pub medals: BTreeSet<Medals>,
    pub picture_file: Utf8PathBuf,
    pub patch_file: Utf8PathBuf,
    pub personal_text: String,
    pub squadron: String,
    pub voice: i16,
}

const FILENAME_LEN: usize = 32;
const PASSWORD_LEN: usize = 10;
const CALLSIGN_LEN: usize = 12;
const PERSONAL_TEXT_LEN: usize = 120;
const COMM_LEN: usize = 12;
const NAME_LEN: usize = 20;

impl Logbook {
    pub fn new(name: String, callsign: String, password: String) -> Result<Self> {
        let options_file = Utf8PathBuf::from(&callsign);

        let commissioned = time::OffsetDateTime::now_local()?.format(
            time::macros::format_description!("[month]/[day]/[year repr:last_two]"),
        )?;

        Ok(Self {
            name,
            callsign,
            password,
            options_file,
            commissioned,
            ..Default::default()
        })
    }

    pub fn parse<R: Read>(r: R) -> Result<Self> {
        let mut r = DecryptRead::new(r, 0x58);

        let mut name_buf = [0; NAME_LEN + 1];
        r.read_exact(&mut name_buf)?;
        let name = buf_to_str(&name_buf)?.to_owned();

        let mut callsign_buf = [0; CALLSIGN_LEN + 1];
        r.read_exact(&mut callsign_buf)?;
        let callsign = buf_to_str(&callsign_buf)?.to_owned();

        let mut pw_buf = [0; PASSWORD_LEN + 1];
        r.read_exact(&mut pw_buf)?;
        xor_password(&mut pw_buf);
        let password = buf_to_str(&pw_buf)?.to_owned();

        let mut commission_buf = [0; COMM_LEN + 1];
        r.read_exact(&mut commission_buf)?;
        let commissioned = buf_to_str(&commission_buf)?.to_owned();

        let mut options_buf = [0; CALLSIGN_LEN + 1];
        r.read_exact(&mut options_buf)?;
        let options_file: Utf8PathBuf = buf_to_str(&options_buf)?.into();

        r.read_exact(&mut [0; 1])?;

        let flight_hours = r.read_f32::<LE>()?;
        let ace_factor = r.read_f32::<LE>()?;

        let rank = Rank::try_from(r.read_i32::<LE>()?)
            .map_err(|e| anyhow!("{} isn't a valid rank index", e.number))?;

        assert_eq!(r.position() % 4, 0);
        let mut dogfight_buf = [0; DogfightStats::BYTE_LEN];
        r.read_exact(&mut dogfight_buf)?;
        let dogfight_stats = DogfightStats::read_bytes(&dogfight_buf);

        assert_eq!(r.position() % 4, 0);
        let mut campaign_buf = [0; CampaignStats::BYTE_LEN];
        r.read_exact(&mut campaign_buf)?;
        let campaign_stats = CampaignStats::read_bytes(&campaign_buf);

        r.read_exact(&mut [0; 2])?;
        assert_eq!(r.position() % 4, 0);

        let mut medals = BTreeSet::default();
        for m in Medals::into_enum_iter() {
            if r.read_u8()? > 0 {
                medals.insert(m);
            }
        }

        r.read_exact(&mut [0; 2])?;
        assert_eq!(r.position() % 4, 0);

        // Skip picture resource ID
        r.read_exact(&mut [0; 4])?;

        let mut picture_buf = [0; FILENAME_LEN + 1];
        r.read_exact(&mut picture_buf)?;
        let picture_file = buf_to_str(&picture_buf)?.into();

        r.read_exact(&mut [0; 3])?;
        assert_eq!(r.position() % 4, 0);

        // Skip patch resource ID
        r.read_exact(&mut [0; 4])?;

        let mut patch_buf = [0; FILENAME_LEN + 1];
        r.read_exact(&mut patch_buf)?;
        let patch_file = buf_to_str(&patch_buf)?.into();

        let mut personal_buf = [0; PERSONAL_TEXT_LEN + 1];
        r.read_exact(&mut personal_buf)?;
        let personal_text = buf_to_str(&personal_buf)?.into();

        let mut squadron_buf = [0; NAME_LEN];
        r.read_exact(&mut squadron_buf)?;
        let squadron = buf_to_str(&squadron_buf)?.into();

        let voice = r.read_i16::<LE>()?;
        ensure!(voice < 12, "voice index {} > 11", voice);

        let checksum = r.read_u32::<LE>()?;
        ensure!(checksum == 0, "Decryption failed - bad checksum");

        Ok(Self {
            name,
            callsign,
            password,
            commissioned,
            options_file,
            flight_hours,
            ace_factor,
            rank,
            dogfight_stats,
            campaign_stats,
            medals,
            picture_file,
            patch_file,
            personal_text,
            squadron,
            voice,
        })
    }

    pub fn write<W: Write>(&self, w: W) -> Result<()> {
        let mut w = EncryptWrite::new(w, 0x58);
        let w = &mut w;

        write_padded(w, &self.name, NAME_LEN + 1)?;
        write_padded(w, &self.callsign, CALLSIGN_LEN + 1)?;

        write_password(w, &self.password)?;

        write_padded(w, &self.commissioned, COMM_LEN + 1)?;
        write_padded(w, &self.options_file, CALLSIGN_LEN + 1)?;
        w.write_all(&[0; 1])?;
        w.write_f32::<LE>(self.flight_hours)?;
        w.write_f32::<LE>(self.ace_factor)?;
        w.write_i32::<LE>(self.rank.into())?;

        assert_eq!(w.position() % 4, 0);
        let mut dogfight_buf = [0; DogfightStats::BYTE_LEN];
        self.dogfight_stats.write_bytes(&mut dogfight_buf);
        w.write_all(&dogfight_buf)?;

        assert_eq!(w.position() % 4, 0);
        let mut campaign_buf = [0; CampaignStats::BYTE_LEN];
        self.campaign_stats.write_bytes(&mut campaign_buf);
        w.write_all(&campaign_buf)?;

        w.write_all(&[0; 2])?;
        assert_eq!(w.position() % 4, 0);

        for m in Medals::into_enum_iter() {
            w.write_all(&[self.medals.contains(&m) as u8])?;
        }

        w.write_all(&[0; 2])?;
        assert_eq!(w.position() % 4, 0);

        // Skip picture resource ID
        w.write_all(&[0; 4])?;

        write_padded(w, &self.picture_file, FILENAME_LEN + 1)?;

        w.write_all(&[0; 3])?;
        assert_eq!(w.position() % 4, 0);

        // Skip patch resource ID
        w.write_all(&[0; 4])?;

        write_padded(w, &self.patch_file, FILENAME_LEN + 1)?;
        write_padded(w, &self.personal_text, PERSONAL_TEXT_LEN + 1)?;
        write_padded(w, &self.squadron, NAME_LEN)?;

        ensure!(self.voice < 12, "voice index {} > 11", self.voice);
        w.write_i16::<LE>(self.voice)?;

        w.write_u32::<LE>(0)?; // "checksum

        Ok(())
    }
}

fn buf_to_str(buf: &[u8]) -> Result<&str> {
    Ok(std::str::from_utf8(buf)?.split('\0').next().unwrap())
}

fn write_padded<W: Write, S: AsRef<str>>(w: &mut W, s: S, pad_to: usize) -> Result<()> {
    let s = s.as_ref();
    ensure!(
        s.len() < pad_to,
        "{s} is longer than the allowed length ({})",
        pad_to - 1
    );

    w.write_all(s.as_bytes())?;
    let padding = vec![0; pad_to - s.len()];
    w.write_all(&padding)?;

    Ok(())
}

const MASTER_KEY: &[u8] = b"Falcon is your Master";

struct DecryptRead<R> {
    inner: R,
    start: u8,
    bytes_read: usize,
}

impl<R: Read> DecryptRead<R> {
    fn new(inner: R, start: u8) -> Self {
        Self {
            inner,
            start,
            bytes_read: 0,
        }
    }

    fn position(&self) -> usize {
        self.bytes_read
    }
}

impl<R: Read> Read for DecryptRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let amount_read = self.inner.read(buf)?;

        for b in &mut buf[..amount_read] {
            let next = *b;
            *b ^= self.start;
            *b ^= MASTER_KEY[self.bytes_read % MASTER_KEY.len()];

            self.bytes_read += 1;
            self.start = next;
        }

        Ok(amount_read)
    }
}

struct EncryptWrite<W> {
    inner: W,
    start: u8,
    bytes_written: usize,
}

impl<W: Write> EncryptWrite<W> {
    fn new(inner: W, start: u8) -> Self {
        Self {
            inner,
            start,
            bytes_written: 0,
        }
    }

    fn position(&self) -> usize {
        self.bytes_written
    }
}

impl<W: Write> Write for EncryptWrite<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut this_write: usize = 0;

        for b in buf {
            let mut to_write = *b;
            to_write ^= MASTER_KEY[self.bytes_written % MASTER_KEY.len()];
            to_write ^= self.start;

            match self.inner.write(&[to_write]) {
                Ok(0) => break,
                Ok(1) => {
                    this_write += 1;
                    self.bytes_written += 1;
                    self.start = to_write;
                }
                Ok(_) => unreachable!(),
                Err(e) => {
                    if this_write == 0 {
                        return Err(e);
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(this_write)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn xor_password(pw: &mut [u8]) {
    const MASK1: &[u8] = b"Who needs a password!";
    const MASK2: &[u8] = b"Repend, Falcon is coming!";

    assert_eq!(pw.len(), PASSWORD_LEN + 1);

    // Despite being XOR'd to hell, the password is null-terminated
    assert_eq!(pw[PASSWORD_LEN], 0);

    for (i, b) in pw.iter_mut().take(PASSWORD_LEN).enumerate() {
        *b ^= MASK1[i % MASK1.len()];
        *b ^= MASK2[i % MASK2.len()];
    }
}

fn write_password<W: Write>(w: &mut W, pw: &str) -> Result<()> {
    ensure!(
        pw.len() <= PASSWORD_LEN,
        "password {pw} is longer than the allowed length ({PASSWORD_LEN})"
    );

    let mut buf: Vec<u8> = pw.as_bytes().to_owned();
    buf.resize(PASSWORD_LEN + 1, 0);
    xor_password(&mut buf);

    w.write_all(&buf)?;

    Ok(())
}
