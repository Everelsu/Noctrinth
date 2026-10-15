package com.modrinth.theseus.agent.skins;

import java.nio.file.Path;
import java.util.Map;

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
     * The shape OptiFine serves: the game's 64 by 32 with the empty part cut off. Redrawn before the
     * game gets it — see {@link CapeImages}.
     */
    private static final int OPTIFINE_WIDTH = 46;

    private static final int OPTIFINE_HEIGHT = 22;

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
        return CapeImages.publish(Http.getBytes(capesUrl + Http.encode(username) + ".png"), directory, username);
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
