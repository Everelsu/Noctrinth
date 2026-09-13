package com.modrinth.theseus.agent.skins;

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
 * OptiFine's capes, which a great many players have and nothing else knows about.
 *
 * <p>Answered by name, which is what makes it usable at all on a world opened to LAN: there the
 * server is in offline mode and everybody's id is derived from their name rather than being the one
 * Mojang issued, so anything that looks players up by id finds nobody.
 *
 * <p>This source has no skins, and could not take part before — the chain took the first source to
 * answer anything, so a cape would have arrived in place of a skin. Answers are merged a texture at
 * a time now, which is what a cape-only source needs. See {@link SkinSource}.
 */
final class OptiFineCapeSource implements SkinSource.Source {
    /** What names this one in {@code noctrinth.skins.source}. */
    static final String NAME = "optifine";

    private static final String CAPES_URL = "http://s.optifine.net/capes/";

    /**
     * The shape OptiFine serves, and the shape Minecraft reads.
     *
     * <p>OptiFine's capes are 46 by 22 and the game's are 64 by 32, laid out so that the smaller one
     * is the top left corner of the larger. Handing the game the URL as it comes would put the cape
     * on backwards and half off the player, so it is redrawn onto a canvas of the size the game
     * expects and served from disk. Larger capes are the same picture at a multiple of that size,
     * so the canvas grows by the same multiple.
     */
    private static final int OPTIFINE_WIDTH = 46;

    private static final int OPTIFINE_HEIGHT = 22;
    private static final int VANILLA_WIDTH = 64;
    private static final int VANILLA_HEIGHT = 32;

    private final String capesUrl;
    private final Path directory;

    OptiFineCapeSource(Path directory) {
        this(CAPES_URL, directory);
    }

    /** The end is an argument so that a test can stand somewhere else in for it. */
    OptiFineCapeSource(String capesUrl, Path directory) {
        this.capesUrl = capesUrl;
        this.directory = directory;
    }

    @Override
    public Map<String, SkinSource.Texture> textures(String username) throws Exception {
        final byte[] original = Http.getBytes(capesUrl + Http.encode(username) + ".png");
        if (original == null || original.length == 0) {
            return Collections.emptyMap();
        }

        final Path file = convert(original, username);
        if (file == null) {
            return Collections.emptyMap();
        }

        final String url = LocalTextures.publish(file);
        if (url == null) {
            return Collections.emptyMap();
        }

        final Map<String, SkinSource.Texture> textures = new LinkedHashMap<>();
        textures.put("CAPE", new SkinSource.Texture(url, Collections.<String, String>emptyMap()));
        return textures;
    }

    /**
     * The cape redrawn at the size the game expects, written where it can be served from.
     *
     * <p>Null when the picture is not one: a cape that cannot be read is not worth a crash, and the
     * player simply has no cape, which is what they had a moment ago.
     */
    private Path convert(byte[] original, String username) throws Exception {
        final BufferedImage source = ImageIO.read(new ByteArrayInputStream(original));
        if (source == null || source.getWidth() <= 0 || source.getHeight() <= 0) {
            SkinSource.debug("OptiFine sent something that is not a picture for " + username);
            return null;
        }

        final BufferedImage cape = onVanillaCanvas(source);

        Files.createDirectories(directory);
        final Path file = directory.resolve(username.toLowerCase(Locale.ROOT) + ".png");

        // Through bytes rather than straight to the file, so that a write that
        // fails halfway cannot leave half a cape behind for the next run to
        // read as a whole one.
        final ByteArrayOutputStream encoded = new ByteArrayOutputStream();
        ImageIO.write(cape, "PNG", encoded);
        Files.write(file, encoded.toByteArray());

        return file;
    }

    /** The picture on a canvas of the size the game reads, at the same scale it arrived in. */
    private static BufferedImage onVanillaCanvas(BufferedImage source) {
        // A cape twice OptiFine's size is drawn on a canvas twice the game's.
        // Worked from the width because that is the side that differs most, and
        // rounded up so a picture between two scales is never cropped.
        final int scale = Math.max(1, (source.getWidth() + OPTIFINE_WIDTH - 1) / OPTIFINE_WIDTH);
        final int width = Math.max(VANILLA_WIDTH * scale, source.getWidth());
        final int height = Math.max(VANILLA_HEIGHT * scale, source.getHeight());

        if (width == source.getWidth() && height == source.getHeight()) {
            // Already the shape the game wants — some capes are served that way.
            return source;
        }

        final BufferedImage canvas = new BufferedImage(width, height, BufferedImage.TYPE_INT_ARGB);
        final java.awt.Graphics2D graphics = canvas.createGraphics();
        try {
            graphics.drawImage(source, 0, 0, null);
        } finally {
            graphics.dispose();
        }

        return canvas;
    }

    /** Whether a picture of this size is one of OptiFine's rather than the game's own. */
    static boolean isOptiFineShape(int width, int height) {
        return width * OPTIFINE_HEIGHT == height * OPTIFINE_WIDTH;
    }

    @Override
    public String toString() {
        return NAME;
    }
}
