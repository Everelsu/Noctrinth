package com.modrinth.theseus.agent.skins;

import java.awt.Graphics2D;
import java.awt.image.BufferedImage;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;
import javax.imageio.ImageIO;

/**
 * Capes from services that do not lay them out the way the game reads them, put back in its shape.
 *
 * <p>The game's cape is a 64 by 32 canvas with the cape itself in the top left 22 by 17 of it. The
 * services cut that canvas down in their own ways — OptiFine to 46 by 22, LabyMod to the 22 by 17
 * of the cape alone — and MinecraftCapes stacks the frames of an animated cape one under another.
 * Each of those handed to the game as it comes is drawn stretched, or backwards and half off the
 * player, so it is redrawn onto a canvas of the game's shape and served from disk. A larger cape is
 * the same picture at a multiple of the size, and the canvas grows by the same multiple.
 */
final class CapeImages {
    private static final int VANILLA_WIDTH = 64;
    private static final int VANILLA_HEIGHT = 32;
    private static final int OPTIFINE_WIDTH = 46;
    private static final int OPTIFINE_HEIGHT = 22;
    private static final int CAPE_WIDTH = 22;
    private static final int CAPE_HEIGHT = 17;

    private CapeImages() {}

    /**
     * The cape as a texture the game can be handed, or an empty map when there is none.
     *
     * <p>{@code key} names the file it is kept under, so the same player's cape from the same
     * service replaces itself rather than piling up.
     */
    static Map<String, SkinSource.Texture> publish(byte[] original, Path directory, String key) throws Exception {
        if (original == null || original.length == 0) {
            return Collections.emptyMap();
        }

        final BufferedImage source = ImageIO.read(new ByteArrayInputStream(original));
        if (source == null || source.getWidth() <= 0 || source.getHeight() <= 0) {
            SkinSource.debug("Got something that is not a picture for the cape " + key);
            return Collections.emptyMap();
        }

        Files.createDirectories(directory);
        final Path file = directory.resolve(key.toLowerCase(Locale.ROOT) + ".png");

        // Through bytes rather than straight to the file, so that a write that
        // fails halfway cannot leave half a cape behind for the next run to
        // read as a whole one.
        final ByteArrayOutputStream encoded = new ByteArrayOutputStream();
        ImageIO.write(onVanillaCanvas(source), "PNG", encoded);
        Files.write(file, encoded.toByteArray());

        final String url = LocalTextures.publish(file);
        if (url == null) {
            return Collections.emptyMap();
        }

        final Map<String, SkinSource.Texture> textures = new LinkedHashMap<>();
        textures.put("CAPE", new SkinSource.Texture(url, Collections.<String, String>emptyMap()));
        return textures;
    }

    /** The picture laid out the way the game reads a cape. */
    static BufferedImage onVanillaCanvas(BufferedImage source) {
        final int width = source.getWidth();
        final int height = source.getHeight();

        if (width * OPTIFINE_HEIGHT == height * OPTIFINE_WIDTH) {
            return inCorner(source, width / OPTIFINE_WIDTH);
        }
        if (width * CAPE_HEIGHT == height * CAPE_WIDTH) {
            return inCorner(source, width / CAPE_WIDTH);
        }
        if (width >= 2 && height > width / 2 && height % (width / 2) == 0) {
            // Frames of an animated cape, one under another: the first is the cape.
            return source.getSubimage(0, 0, width, width / 2);
        }

        return source;
    }

    /** The picture in the top left of a game-shaped canvas at {@code scale}. */
    private static BufferedImage inCorner(BufferedImage source, int scale) {
        final int size = Math.max(1, scale);
        final BufferedImage canvas = new BufferedImage(
                Math.max(VANILLA_WIDTH * size, source.getWidth()),
                Math.max(VANILLA_HEIGHT * size, source.getHeight()),
                BufferedImage.TYPE_INT_ARGB);
        final Graphics2D graphics = canvas.createGraphics();
        try {
            graphics.drawImage(source, 0, 0, null);
        } finally {
            graphics.dispose();
        }

        return canvas;
    }
}
