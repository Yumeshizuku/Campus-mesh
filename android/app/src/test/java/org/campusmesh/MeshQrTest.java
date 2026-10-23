package org.campusmesh;

import static org.junit.Assert.assertEquals;

import com.google.zxing.BinaryBitmap;
import com.google.zxing.MultiFormatReader;
import com.google.zxing.RGBLuminanceSource;
import com.google.zxing.common.BitMatrix;
import com.google.zxing.common.HybridBinarizer;
import org.junit.Test;

public class MeshQrTest {
    @Test
    public void signedCardsAndInvitesPreserveUnicodeExactly() throws Exception {
        String[] cards = {
            "{\"kind\":\"campusmesh.contact\",\"version\":1,\"device_id\":\"" + "a".repeat(64)
                + "\",\"contact\":{\"nickname\":\"同学🌙\",\"sign\":\"" + "b".repeat(64)
                + "\",\"exchange\":\"" + "c".repeat(64) + "\"},\"signature\":\"" + "d".repeat(128) + "\"}",
            "{\"kind\":\"campusmesh.group\",\"version\":1,\"group_name\":\"图书馆·自习小组📚\",\"owner\":{\"nickname\":\"群主\"},\"signature\":\"" + "e".repeat(128) + "\"}"
        };
        for (String card : cards) {
            BitMatrix image = MeshQr.encode(card, 720);
            int width = image.getWidth(), height = image.getHeight();
            int[] pixels = new int[width * height];
            for (int y = 0; y < height; y++) {
                for (int x = 0; x < width; x++) {
                    pixels[y * width + x] = image.get(x, y) ? 0xFF000000 : 0xFFFFFFFF;
                }
            }
            BinaryBitmap bitmap = new BinaryBitmap(new HybridBinarizer(new RGBLuminanceSource(width, height, pixels)));
            assertEquals(card, new MultiFormatReader().decode(bitmap).getText());
        }
    }
}
