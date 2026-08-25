package com.modrinth.theseus.agent.skins;

import java.nio.file.DirectoryStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;

/**
 * Skins put on disk by hand, for players nothing online has heard of.
 *
 * <p>A folder of the launcher's own, one {@code <name>.png} per player, with {@code capes/} and
 * {@code elytras/} beside it for the rest. It is asked before anything on the network, so it is
 * also how somebody overrides a skin they do not like, and it costs nothing — no request, no wait.
 */
final class LocalSource implements SkinSource.Source {
    private static final String SKIN = "SKIN";

    private final Path directory;

    LocalSource(Path directory) {
        this.directory = directory;
    }

    @Override
    public Map<String, SkinSource.Texture> textures(String username) throws Exception {
        final Map<String, SkinSource.Texture> textures = new LinkedHashMap<>();

        put(textures, SKIN, directory, username);
        put(textures, "CAPE", directory.resolve("capes"), username);
        put(textures, "ELYTRA", directory.resolve("elytras"), username);

        return textures;
    }

    private void put(Map<String, SkinSource.Texture> textures, String type, Path folder, String username)
            throws Exception {
        final Path file = find(folder, username);
        if (file == null) {
            return;
        }

        final String url = LocalTextures.publish(file);
        if (url == null) {
            return;
        }

        // A skin drawn for the slim model is named for it, since a file on disk
        // has nobody to ask about which arms it was made for.
        final Map<String, String> metadata = new LinkedHashMap<>();
        if (SKIN.equals(type)
                && file.getFileName().toString().toLowerCase(Locale.ROOT).contains("-slim.")) {
            metadata.put("model", "slim");
        }

        textures.put(type, new SkinSource.Texture(url, metadata));
    }

    /**
     * The file for this player, if there is one.
     *
     * <p>Names are matched without regard to case, because whoever named the file was going by how
     * the player writes their own name and the server may not agree.
     */
    private static Path find(Path folder, String username) throws Exception {
        if (!Files.isDirectory(folder)) {
            return null;
        }

        final String wanted = username.toLowerCase(Locale.ROOT);
        try (DirectoryStream<Path> files = Files.newDirectoryStream(folder)) {
            for (final Path file : files) {
                if (!Files.isRegularFile(file)) {
                    continue;
                }

                final String name = file.getFileName().toString().toLowerCase(Locale.ROOT);
                if (!name.endsWith(".png")) {
                    continue;
                }

                final String stem = name.substring(0, name.length() - 4);
                if (stem.equals(wanted) || stem.equals(wanted + "-slim")) {
                    return file;
                }
            }
        }

        return null;
    }

    @Override
    public String toString() {
        return directory.toString();
    }
}
