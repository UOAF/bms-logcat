use std::{
    collections::BTreeSet,
    fs::File,
    io::{prelude::*, BufReader},
};

use anyhow::{anyhow, ensure, Context, Result};
use byte_struct::*;
use byteorder::{ReadBytesExt, LE};
use camino::{Utf8Path, Utf8PathBuf};
use enum_iterator::IntoEnumIterator;
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Debug, Copy, Clone, IntoPrimitive, TryFromPrimitive)]
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

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, IntoEnumIterator)]
pub enum Medals {
    AirForceCross,
    SilverStar,
    DistinguishedFlyingCross,
    AirMedal,
    KoreaCampaign,
    Longevity,
}

#[derive(Debug, Default, ByteStruct)]
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

#[derive(Debug, Default, ByteStruct)]
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

#[derive(Debug)]
pub struct Logbook {
    pub name: String,
    pub callsign: String,
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

impl Logbook {
    fn parse<R: Read>(mut r: DecryptRead<R>) -> Result<Self> {
        const FILENAME_LEN: usize = 32;
        const PASSWORD_LEN: usize = 10;
        const CALLSIGN_LEN: usize = 12;
        const PERSONAL_TEXT_LEN: usize = 120;
        const COMM_LEN: usize = 12;
        const NAME_LEN: usize = 20;

        let mut name_buf = [0; NAME_LEN + 1];
        r.read_exact(&mut name_buf)?;
        let name = buf_to_str(&name_buf)?.to_owned();

        let mut callsign_buf = [0; CALLSIGN_LEN + 1];
        r.read_exact(&mut callsign_buf)?;
        let callsign = buf_to_str(&callsign_buf)?.to_owned();

        r.read_exact(&mut [0; PASSWORD_LEN + 1])?;

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
}

fn buf_to_str(buf: &[u8]) -> Result<&str> {
    Ok(std::str::from_utf8(buf)?.trim_end_matches('\0'))
}

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

        const MASTER_KEY: &[u8] = b"Falcon is your Master";

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

pub fn read(path: &Utf8Path) -> Result<Logbook> {
    let reader: Box<dyn Read> = match path.as_str() {
        "-" => Box::new(std::io::stdin()),
        p => {
            let f = File::open(p).with_context(|| format!("Couldn't open {}", p))?;
            Box::new(f)
        }
    };

    let decryptor = DecryptRead::new(BufReader::new(reader), 0x58);

    Logbook::parse(decryptor)
}