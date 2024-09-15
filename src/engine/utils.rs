pub fn crypt(buf: &mut [u8]) {
    buf.iter_mut().for_each(|ch| *ch = !*ch);
}

pub fn skip_sinister_header(mut r: impl std::io::Read + std::io::Seek) -> std::io::Result<()> {
    use byteorder::ReadBytesExt;
    use std::io::SeekFrom;

    fn skip_single_line(mut r: impl std::io::Read + std::io::Seek) -> std::io::Result<()> {
        loop {
            let ch = r.read_u8()?;

            if ch == 10 || ch == 13 {
                let next = r.read_u8()?;
                if next == 10 || next == 13 {
                    break;
                } else {
                    // If the second character in the sequence is not a newline, reverse it.
                    r.seek(SeekFrom::Current(-1))?;
                    break;
                }
            }
        }

        Ok(())
    }

    loop {
        let ch = r.read_u8()?;
        if ch == b'*' {
            skip_single_line(&mut r)?;
        } else {
            // Not a comment line, so reverse the character we read.
            r.seek(SeekFrom::Current(-1))?;
            break;
        }
    }

    Ok(())
}
