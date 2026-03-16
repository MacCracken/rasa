use rasa_core::color::{
    Color, IccProfile, ProfileColorSpace, linear_to_srgb, rgb_to_cmyk_naive, srgb_to_linear,
};
use rasa_core::error::RasaError;
use rasa_core::pixel::PixelBuffer;

/// Convert a PixelBuffer to CMYK bytes using an ICC profile transform.
///
/// Returns 4 bytes per pixel (C, M, Y, K) in 0-255 range.
/// `source_profile` should be an RGB profile (e.g. sRGB).
/// `dest_profile` should be a CMYK output profile.
pub fn buffer_to_cmyk_icc(
    buf: &PixelBuffer,
    source_profile: &IccProfile,
    dest_profile: &IccProfile,
) -> Result<Vec<u8>, RasaError> {
    if source_profile.color_space != ProfileColorSpace::Rgb {
        return Err(RasaError::ColorConversionFailed(
            "source profile must be RGB".into(),
        ));
    }
    if dest_profile.color_space != ProfileColorSpace::Cmyk {
        return Err(RasaError::ColorConversionFailed(
            "destination profile must be CMYK".into(),
        ));
    }

    let src = lcms2::Profile::new_icc(source_profile.data())
        .map_err(|e| RasaError::ColorConversionFailed(format!("source profile: {e}")))?;
    let dst = lcms2::Profile::new_icc(dest_profile.data())
        .map_err(|e| RasaError::ColorConversionFailed(format!("dest profile: {e}")))?;

    let transform = lcms2::Transform::new(
        &src,
        lcms2::PixelFormat::RGB_8,
        &dst,
        lcms2::PixelFormat::CMYK_8,
        lcms2::Intent::RelativeColorimetric,
    )
    .map_err(|e| RasaError::ColorConversionFailed(format!("transform: {e}")))?;

    // Prepare sRGB u8 input
    let pixels = buf.pixels();
    let mut rgb_input: Vec<u8> = Vec::with_capacity(pixels.len() * 3);
    for px in pixels {
        rgb_input.push((linear_to_srgb(px.r) * 255.0 + 0.5) as u8);
        rgb_input.push((linear_to_srgb(px.g) * 255.0 + 0.5) as u8);
        rgb_input.push((linear_to_srgb(px.b) * 255.0 + 0.5) as u8);
    }

    let mut cmyk_output = vec![0u8; pixels.len() * 4];
    transform.transform_pixels(&rgb_input, &mut cmyk_output);

    Ok(cmyk_output)
}

/// Convert CMYK bytes to a PixelBuffer using ICC profiles.
pub fn cmyk_to_buffer_icc(
    cmyk_data: &[u8],
    width: u32,
    height: u32,
    source_profile: &IccProfile,
    dest_profile: &IccProfile,
) -> Result<PixelBuffer, RasaError> {
    if source_profile.color_space != ProfileColorSpace::Cmyk {
        return Err(RasaError::ColorConversionFailed(
            "source profile must be CMYK".into(),
        ));
    }
    if dest_profile.color_space != ProfileColorSpace::Rgb {
        return Err(RasaError::ColorConversionFailed(
            "destination profile must be RGB".into(),
        ));
    }

    let pixel_count = (width as usize) * (height as usize);
    let expected = pixel_count * 4;
    if cmyk_data.len() != expected {
        return Err(RasaError::InvalidDimensions { width, height });
    }

    let src = lcms2::Profile::new_icc(source_profile.data())
        .map_err(|e| RasaError::ColorConversionFailed(format!("source profile: {e}")))?;
    let dst = lcms2::Profile::new_icc(dest_profile.data())
        .map_err(|e| RasaError::ColorConversionFailed(format!("dest profile: {e}")))?;

    let transform = lcms2::Transform::new(
        &src,
        lcms2::PixelFormat::CMYK_8,
        &dst,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::RelativeColorimetric,
    )
    .map_err(|e| RasaError::ColorConversionFailed(format!("transform: {e}")))?;

    let mut rgb_output = vec![0u8; pixel_count * 3];
    transform.transform_pixels(cmyk_data, &mut rgb_output);

    let mut buf = PixelBuffer::new(width, height);
    let pixels = buf.pixels_mut();
    for (i, px) in pixels.iter_mut().enumerate() {
        let offset = i * 3;
        *px = Color::new(
            srgb_to_linear(rgb_output[offset] as f32 / 255.0),
            srgb_to_linear(rgb_output[offset + 1] as f32 / 255.0),
            srgb_to_linear(rgb_output[offset + 2] as f32 / 255.0),
            1.0,
        );
    }

    Ok(buf)
}

/// Apply an ICC profile transform between two RGB profiles on a PixelBuffer.
pub fn apply_profile_transform(
    buf: &mut PixelBuffer,
    source: &IccProfile,
    dest: &IccProfile,
) -> Result<(), RasaError> {
    if source.color_space != ProfileColorSpace::Rgb || dest.color_space != ProfileColorSpace::Rgb {
        return Err(RasaError::ColorConversionFailed(
            "both profiles must be RGB for in-place transform".into(),
        ));
    }

    let src = lcms2::Profile::new_icc(source.data())
        .map_err(|e| RasaError::ColorConversionFailed(format!("source profile: {e}")))?;
    let dst = lcms2::Profile::new_icc(dest.data())
        .map_err(|e| RasaError::ColorConversionFailed(format!("dest profile: {e}")))?;

    let transform = lcms2::Transform::new(
        &src,
        lcms2::PixelFormat::RGB_8,
        &dst,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::RelativeColorimetric,
    )
    .map_err(|e| RasaError::ColorConversionFailed(format!("transform: {e}")))?;

    let pixels = buf.pixels_mut();
    let mut rgb_data: Vec<u8> = Vec::with_capacity(pixels.len() * 3);
    for px in pixels.iter() {
        rgb_data.push((linear_to_srgb(px.r) * 255.0 + 0.5) as u8);
        rgb_data.push((linear_to_srgb(px.g) * 255.0 + 0.5) as u8);
        rgb_data.push((linear_to_srgb(px.b) * 255.0 + 0.5) as u8);
    }

    let mut rgb_out = vec![0u8; pixels.len() * 3];
    transform.transform_pixels(&rgb_data, &mut rgb_out);

    for (i, px) in pixels.iter_mut().enumerate() {
        let offset = i * 3;
        px.r = srgb_to_linear(rgb_out[offset] as f32 / 255.0);
        px.g = srgb_to_linear(rgb_out[offset + 1] as f32 / 255.0);
        px.b = srgb_to_linear(rgb_out[offset + 2] as f32 / 255.0);
    }

    Ok(())
}

/// Naive (no ICC) RGB-to-CMYK conversion for an entire PixelBuffer.
///
/// Returns 4 bytes per pixel (C, M, Y, K) in 0-255 range.
pub fn buffer_to_cmyk_naive(buf: &PixelBuffer) -> Vec<u8> {
    let pixels = buf.pixels();
    let mut out = Vec::with_capacity(pixels.len() * 4);
    for px in pixels {
        let r = linear_to_srgb(px.r);
        let g = linear_to_srgb(px.g);
        let b = linear_to_srgb(px.b);
        let cmyk = rgb_to_cmyk_naive(r, g, b);
        out.push((cmyk.c * 255.0 + 0.5) as u8);
        out.push((cmyk.m * 255.0 + 0.5) as u8);
        out.push((cmyk.y * 255.0 + 0.5) as u8);
        out.push((cmyk.k * 255.0 + 0.5) as u8);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_to_cmyk_naive_white_pixel() {
        let buf = PixelBuffer::filled(1, 1, Color::WHITE);
        let cmyk = buffer_to_cmyk_naive(&buf);
        assert_eq!(cmyk.len(), 4);
        // White = C0 M0 Y0 K0
        assert_eq!(cmyk[0], 0); // C
        assert_eq!(cmyk[1], 0); // M
        assert_eq!(cmyk[2], 0); // Y
        assert_eq!(cmyk[3], 0); // K
    }

    #[test]
    fn buffer_to_cmyk_naive_black_pixel() {
        let buf = PixelBuffer::filled(1, 1, Color::BLACK);
        let cmyk = buffer_to_cmyk_naive(&buf);
        assert_eq!(cmyk[3], 255); // K = full black
    }

    #[test]
    fn buffer_to_cmyk_naive_dimensions() {
        let buf = PixelBuffer::filled(10, 5, Color::WHITE);
        let cmyk = buffer_to_cmyk_naive(&buf);
        assert_eq!(cmyk.len(), 10 * 5 * 4);
    }

    #[test]
    fn buffer_to_cmyk_icc_validates_source() {
        let buf = PixelBuffer::filled(1, 1, Color::WHITE);
        let mut cmyk_header = vec![0u8; 128];
        cmyk_header[16..20].copy_from_slice(b"CMYK");
        let cmyk_prof = IccProfile::from_bytes(cmyk_header).unwrap();

        // Source must be RGB, not CMYK
        let result = buffer_to_cmyk_icc(&buf, &cmyk_prof, &cmyk_prof);
        assert!(result.is_err());
    }

    #[test]
    fn buffer_to_cmyk_icc_validates_dest() {
        let buf = PixelBuffer::filled(1, 1, Color::WHITE);
        let srgb = IccProfile::srgb_v2();

        // Dest must be CMYK, not RGB
        let result = buffer_to_cmyk_icc(&buf, &srgb, &srgb);
        assert!(result.is_err());
    }

    #[test]
    fn cmyk_to_buffer_icc_validates_source() {
        let data = vec![0u8; 4];
        let srgb = IccProfile::srgb_v2();

        // Source must be CMYK
        let result = cmyk_to_buffer_icc(&data, 1, 1, &srgb, &srgb);
        assert!(result.is_err());
    }

    #[test]
    fn apply_profile_transform_validates_rgb() {
        let mut buf = PixelBuffer::filled(1, 1, Color::WHITE);
        let mut cmyk_header = vec![0u8; 128];
        cmyk_header[16..20].copy_from_slice(b"CMYK");
        let cmyk_prof = IccProfile::from_bytes(cmyk_header).unwrap();
        let srgb = IccProfile::srgb_v2();

        let result = apply_profile_transform(&mut buf, &srgb, &cmyk_prof);
        assert!(result.is_err());
    }

    #[test]
    fn apply_profile_transform_srgb_to_srgb() {
        let mut buf = PixelBuffer::filled(2, 2, Color::new(0.5, 0.3, 0.8, 1.0));
        let srgb = IccProfile::srgb_v2();

        // sRGB -> sRGB should be near-identity
        let original_px = buf.get(0, 0).unwrap();
        let result = apply_profile_transform(&mut buf, &srgb, &srgb);
        assert!(result.is_ok());

        let transformed_px = buf.get(0, 0).unwrap();
        // Allow tolerance for u8 quantization roundtrip
        assert!((original_px.r - transformed_px.r).abs() < 0.05);
        assert!((original_px.g - transformed_px.g).abs() < 0.05);
        assert!((original_px.b - transformed_px.b).abs() < 0.05);
    }
}
