package com.modrinth.theseus.agent.skins;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import java.nio.file.Path;
import java.util.Collections;
import java.util.Map;

/**
 * Capes from the services whose own mods draw them — LabyMod, MinecraftCapes and SkinMC — for the
 * players who do not have those mods.
 *
 * <p>All three are asked by the id Mojang issued, so they only ever answer for a name somebody has
 * bought; that id is looked up the same way, and from the same memory, as {@link MojangSource}
 * does. They come after every other source, which keeps what the game shows the same as the
 * launcher's preview: the first cape found among Ely.by, Mojang, OptiFine and then these.
 */
final class CapeServiceSource implements SkinSource.Source {
    static final String LABYMOD = "labymod";
    static final String MINECRAFT_CAPES = "minecraftcapes";
    static final String SKINMC = "skinmc";

    private final String name;
    private final Path directory;

    private CapeServiceSource(String name, Path directory) {
        this.name = name;
        this.directory = directory;
    }

    /** The source {@code name} stands for, or null when it is not one of these. */
    static CapeServiceSource named(String name, Path directory) {
        final String lower = name.toLowerCase(java.util.Locale.ROOT);
        if (LABYMOD.equals(lower) || MINECRAFT_CAPES.equals(lower) || SKINMC.equals(lower)) {
            return new CapeServiceSource(lower, directory);
        }
        return null;
    }

    @Override
    public Map<String, SkinSource.Texture> textures(String username) throws Exception {
        final String id = MojangSource.idOf(username);
        if (id == null) {
            return Collections.emptyMap();
        }

        final byte[] cape = fetch(id);
        return cape == null
                ? Collections.<String, SkinSource.Texture>emptyMap()
                : CapeImages.publish(cape, directory, name + "-" + username);
    }

    private byte[] fetch(String id) throws Exception {
        switch (name) {
            case LABYMOD:
                return Http.getBytes("https://dl.labymod.net/capes/" + dashed(id));
            case SKINMC:
                return Http.getBytes("https://skinmc.net/api/v1/skinmcCape/" + dashed(id));
            default:
                // MinecraftCapes stopped serving the picture at /profile/{id}/cape
                // and names its address in the profile instead.
                final String url = capeUrl(Http.getJson("https://api.minecraftcapes.net/profile/" + id));
                return url == null ? null : Http.getBytes(url);
        }
    }

    /** The cape a MinecraftCapes profile points at, the still one before the animated one. */
    static String capeUrl(JsonElement profile) {
        if (profile == null || !profile.isJsonObject()) {
            return null;
        }

        final JsonObject json = profile.getAsJsonObject();
        for (final String key : new String[] {"cape_url", "animated_cape_url"}) {
            final JsonElement value = json.get(key);
            if (value != null && value.isJsonPrimitive() && !value.getAsString().isEmpty()) {
                return value.getAsString();
            }
        }

        return null;
    }

    static String dashed(String id) {
        if (id.length() != 32) {
            return id;
        }
        return id.substring(0, 8) + "-" + id.substring(8, 12) + "-" + id.substring(12, 16) + "-" + id.substring(16, 20)
                + "-" + id.substring(20);
    }

    @Override
    public String toString() {
        return name;
    }
}
