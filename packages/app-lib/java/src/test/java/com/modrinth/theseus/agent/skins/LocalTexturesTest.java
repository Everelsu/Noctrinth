package com.modrinth.theseus.agent.skins;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * What the game may be handed as a texture URL. authlib parses it with {@code java.net.URL} on the
 * render thread without catching anything, so one it cannot parse crashes the game on joining a
 * world.
 */
class LocalTexturesTest {
    @Test
    void takesWhatTheGameCanDownload() {
        assertTrue(LocalTextures.isFetchable("http://textures.minecraft.net/texture/3e459185b026e35e727b"));
        assertTrue(LocalTextures.isFetchable("https://littleskin.cn/textures/abc"));
        assertTrue(LocalTextures.isFetchable("http://[0:0:0:0:0:0:0:1]:52551/rabc.png"));
    }

    @Test
    void refusesWhatWouldCrashIt() {
        assertFalse(LocalTextures.isFetchable(null));
        assertFalse(LocalTextures.isFetchable(""));
        assertFalse(LocalTextures.isFetchable("textures/abc.png"));
        assertFalse(LocalTextures.isFetchable("file:/C:/skins/abc.png"));
        // An IPv6 loopback address written out without brackets, as the relay used to.
        assertFalse(LocalTextures.isFetchable("http://0:0:0:0:0:0:0:1:52551/rabc.png"));
    }

    @Test
    void servesFromAnAddressTheGameCanParse(@TempDir Path folder) throws Exception {
        final Path file = folder.resolve("skin.png");
        Files.write(file, new byte[] {1, 2, 3});

        final String url = LocalTextures.publish(file);
        assertTrue(LocalTextures.isFetchable(url), url);
        assertTrue(LocalTextures.isFetchable(LocalTextures.relay("http://textures.minecraft.net/texture/abc")));
    }
}
