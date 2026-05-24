//! Local-socket framing for the management API.
//!
//! The R2-HIVE spec §5.2 says the socket speaks R2-WIRE. SOCK_STREAM has no
//! message boundaries, so frames are length-prefixed: 4 bytes big-endian length,
//! followed by that many bytes of R2-WIRE extended-format frame.
//!
//! This is a local transport detail — the R2-WIRE payload inside is the same
//! format any mesh peer would see.

use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Max local frame size. Management events are tiny; 64 KiB is plenty.
pub const MAX_FRAME: u32 = 65_536;

/// Read a length-prefixed frame from an async reader. Returns `None` on clean EOF.
pub async fn read_frame<R: AsyncReadExt + Unpin>(r: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut hdr = [0u8; 4];
    match r.read_exact(&mut hdr).await {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_be_bytes(hdr);
    if len == 0 || len > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid local frame length: {len}"),
        ));
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf).await?;
    Ok(Some(buf))
}

/// Write a length-prefixed frame to an async writer.
pub async fn write_frame<W: AsyncWriteExt + Unpin>(w: &mut W, frame: &[u8]) -> io::Result<()> {
    let len = u32::try_from(frame.len()).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, "frame too large for u32 length")
    })?;
    if len > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("frame exceeds MAX_FRAME ({len} > {MAX_FRAME})"),
        ));
    }
    w.write_all(&len.to_be_bytes()).await?;
    w.write_all(frame).await?;
    w.flush().await?;
    Ok(())
}
