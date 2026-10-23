package org.campusmesh;

import com.google.zxing.BarcodeFormat;
import com.google.zxing.EncodeHintType;
import com.google.zxing.MultiFormatWriter;
import com.google.zxing.WriterException;
import com.google.zxing.common.BitMatrix;
import java.util.Collections;

/** Preserve the exact signed JSON, including Chinese and emoji, in every QR card. */
public final class MeshQr {
    private MeshQr() {}

    public static BitMatrix encode(String text, int size) throws WriterException {
        return new MultiFormatWriter().encode(text, BarcodeFormat.QR_CODE, size, size,
                Collections.singletonMap(EncodeHintType.CHARACTER_SET, "UTF-8"));
    }
}
