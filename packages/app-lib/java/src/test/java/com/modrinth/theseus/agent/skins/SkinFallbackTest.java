package com.modrinth.theseus.agent.skins;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.modrinth.theseus.agent.transformers.SessionServiceTransformer;
import com.mojang.authlib.GameProfile;
import com.mojang.authlib.minecraft.InsecureTextureException;
import com.mojang.authlib.minecraft.MinecraftProfileTexture;
import com.mojang.authlib.minecraft.MinecraftProfileTextures;
import com.mojang.authlib.yggdrasil.YggdrasilMinecraftSessionService;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.lang.reflect.InvocationTargetException;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.Charset;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassWriter;

/**
 * End-to-end cover for the skin fallback: the transformer's rewrite, the hook's reflection, and the
 * lookup itself, against a skin system stubbed out on a local socket.
 *
 * <p>What it cannot cover is the game: whether a given Minecraft build calls {@code getTextures} at
 * all is the game's business, and it does. What it does pin down is that a profile arriving without
 * textures leaves with them, that one arriving with textures nobody here can verify does too rather
 * than throwing its way to Steve, that one arriving with textures that are fine is left exactly as
 * it was, and that all of this holds for both places authlib has kept {@code getTextures}.
 */
class SkinFallbackTest {
    private static final String TARGET = "com.mojang.authlib.yggdrasil.YggdrasilMinecraftSessionService";
    private static final String INTERFACE_TARGET = "com.mojang.authlib.minecraft.MinecraftSessionService";
    private static final String INTERFACE_IMPLEMENTATION = "com.mojang.authlib.yggdrasil.YggdrasilSessionService";

    private static final String LOOKED_UP_SKIN = "http://textures.example/looked-up.png";
    private static final String RESPONSE =
            "{\"SKIN\":{\"url\":\"" + LOOKED_UP_SKIN + "\",\"metadata\":{\"model\":\"slim\"}}}";

    private static final List<String> requestedPaths = Collections.synchronizedList(new ArrayList<>());
    private static final List<String> firstSourcePaths = Collections.synchronizedList(new ArrayList<>());

    /** Knows nobody, so every lookup has to fall through to the one after it. */
    private static ServerSocket emptySource;

    private static ServerSocket server;
    private static Thread emptySourceThread;
    private static Thread serverThread;
    private static Object service;
    private static Object interfaceService;

    @BeforeAll
    static void startStubSkinSystem() throws Exception {
        emptySource = new ServerSocket(0, 0, InetAddress.getByName("127.0.0.1"));
        emptySourceThread = new Thread(() -> serve(emptySource, firstSourcePaths, null), "stub-skin-system-empty");
        emptySourceThread.setDaemon(true);
        emptySourceThread.start();

        server = new ServerSocket(0, 0, InetAddress.getByName("127.0.0.1"));
        serverThread = new Thread(() -> serve(server, requestedPaths, RESPONSE), "stub-skin-system");
        serverThread.setDaemon(true);
        serverThread.start();

        // Read by SkinSource when it is first touched, which is why this has to
        // happen before the hook runs for the first time.
        System.setProperty(
                "noctrinth.skins.source",
                "http://127.0.0.1:" + emptySource.getLocalPort() + ",http://127.0.0.1:" + server.getLocalPort());

        service = transformedService();
        interfaceService = transformedInterfaceService();
    }

    @AfterAll
    static void stopStubSkinSystem() throws Exception {
        emptySource.close();
        server.close();
        emptySourceThread.join(2000);
        serverThread.join(2000);
        System.clearProperty("noctrinth.skins.source");
    }

    @Test
    void fillsInTexturesTheServerDidNotSend() throws Exception {
        final MinecraftProfileTexture skin = skinOf(getTextures(new GameProfile("Offline_One", false), true));

        assertNotNull(skin, "a profile with no textures should have come back with one");
        assertEquals(LOOKED_UP_SKIN, skin.getUrl());
        assertEquals("slim", skin.getMetadata("model"), "slim arms should survive the round trip");
        assertTrue(requestedPaths.contains("/textures/Offline_One"), "the lookup should go by name");
        assertTrue(
                firstSourcePaths.contains("/textures/Offline_One"),
                "the source listed first should have been asked first");
    }

    @Test
    void leavesTexturesTheServerSentAlone() throws Exception {
        final MinecraftProfileTexture skin = skinOf(getTextures(new GameProfile("Signed_One", true), true));

        assertEquals(
                YggdrasilMinecraftSessionService.SERVER_SKIN,
                skin.getUrl(),
                "a signed skin must not be second-guessed");
    }

    /**
     * The cross-account case, and the reason the wrapper catches: an Ely.by player's textures are
     * signed by a key a licensed client does not have, so authlib throws instead of returning and
     * the game reads that as no skin at all.
     */
    @Test
    void fillsInTexturesAuthlibRefusedToHandOver() throws Exception {
        final MinecraftProfileTexture skin = skinOf(getTextures(GameProfile.unverifiable("Insecure_One"), true));

        assertNotNull(skin, "a profile whose textures could not be verified should still get one");
        assertEquals(LOOKED_UP_SKIN, skin.getUrl());
    }

    /** With nothing to put in its place, the caller has to see exactly what it would have seen. */
    @Test
    void passesTheFailureOnWhenTheNameCannotBeAskedAbout() {
        final InvocationTargetException thrown = assertThrows(
                InvocationTargetException.class, () -> getTextures(GameProfile.unverifiable("no lookup"), true));

        assertTrue(
                thrown.getCause() instanceof InsecureTextureException,
                "the original failure should have been passed on, not swallowed: " + thrown.getCause());
    }

    /** Ely.by hands out names Mojang would not, and they are still worth asking about. */
    @Test
    void asksAboutNamesMojangWouldNotIssue() throws Exception {
        getTextures(new GameProfile("Ely-By.Guy", false), true);

        assertTrue(requestedPaths.contains("/textures/Ely-By.Guy"), "a name with a dash should have been looked up");
    }

    @Test
    void fillsInTheModernTexturesObject() throws Exception {
        final MinecraftProfileTextures textures =
                (MinecraftProfileTextures) getTextures(new GameProfile("Offline_Two", false));

        assertNotNull(textures.skin(), "a profile with no textures should have come back with one");
        assertEquals(LOOKED_UP_SKIN, textures.skin().getUrl());
        assertNull(textures.cape(), "nothing was said about a cape");
        assertEquals(
                MinecraftProfileTextures.SignatureState.UNSIGNED,
                textures.signatureState(),
                "textures we resolved ourselves are not signed by anyone");
    }

    @Test
    void leavesTheModernTexturesObjectAloneWhenItIsFilled() throws Exception {
        final MinecraftProfileTextures textures =
                (MinecraftProfileTextures) getTextures(new GameProfile("Signed_Two", true));

        assertEquals(
                YggdrasilMinecraftSessionService.SERVER_SKIN, textures.skin().getUrl());
        assertEquals(MinecraftProfileTextures.SignatureState.SIGNED, textures.signatureState());
    }

    /** From 1.20.2 on, {@code getTextures} is a default method on the interface instead. */
    @Test
    void fillsInTexturesWhereTheDefaultMethodLives() throws Exception {
        final MinecraftProfileTextures textures = (MinecraftProfileTextures) interfaceService
                .getClass()
                .getMethod("getTextures", GameProfile.class)
                .invoke(interfaceService, new GameProfile("Offline_Three", false));

        assertNotNull(textures.skin(), "the interface's default method should have been wrapped too");
        assertEquals(LOOKED_UP_SKIN, textures.skin().getUrl());
    }

    @Test
    void leavesTheInterfaceAloneWhenTheServerAnswered() throws Exception {
        final MinecraftProfileTextures textures = (MinecraftProfileTextures) interfaceService
                .getClass()
                .getMethod("getTextures", GameProfile.class)
                .invoke(interfaceService, new GameProfile("Signed_Three", true));

        assertEquals(
                YggdrasilMinecraftSessionService.SERVER_SKIN, textures.skin().getUrl());
    }

    @Test
    void asksAboutEachNameOnlyOnce() throws Exception {
        getTextures(new GameProfile("Cached_One", false), true);
        final int afterFirst = countRequests("/textures/Cached_One");
        getTextures(new GameProfile("Cached_One", false), true);

        assertEquals(afterFirst, countRequests("/textures/Cached_One"), "the second lookup should be cached");
    }

    @SuppressWarnings("unchecked")
    private static MinecraftProfileTexture skinOf(Object textures) {
        return ((Map<MinecraftProfileTexture.Type, MinecraftProfileTexture>) textures)
                .get(MinecraftProfileTexture.Type.SKIN);
    }

    private static Object getTextures(GameProfile profile, boolean requireSecure) throws Exception {
        return service.getClass()
                .getMethod("getTextures", GameProfile.class, boolean.class)
                .invoke(service, profile, requireSecure);
    }

    private static Object getTextures(GameProfile profile) throws Exception {
        return service.getClass().getMethod("getTextures", GameProfile.class).invoke(service, profile);
    }

    private static int countRequests(String path) {
        int count = 0;
        synchronized (requestedPaths) {
            for (final String requested : requestedPaths) {
                if (requested.equals(path)) {
                    count++;
                }
            }
        }
        return count;
    }

    /** The patched session service, loaded apart from the one on the test classpath. */
    private static Object transformedService() throws Exception {
        final Map<String, byte[]> classes = new LinkedHashMap<>();
        classes.put(TARGET, transform(TARGET));

        return loaderFor(classes).loadClass(TARGET).getDeclaredConstructor().newInstance();
    }

    /**
     * The same, for the arrangement from 1.20.2 on.
     *
     * <p>The implementation is loaded alongside the interface rather than shared with the test, so
     * that it inherits the patched default method rather than the one on the test classpath.
     */
    private static Object transformedInterfaceService() throws Exception {
        final Map<String, byte[]> classes = new LinkedHashMap<>();
        classes.put(INTERFACE_TARGET, transform(INTERFACE_TARGET));
        classes.put(INTERFACE_IMPLEMENTATION, readBytes(resourceName(INTERFACE_IMPLEMENTATION)));

        return loaderFor(classes)
                .loadClass(INTERFACE_IMPLEMENTATION)
                .getDeclaredConstructor()
                .newInstance();
    }

    private static byte[] transform(String className) throws Exception {
        final ClassReader reader = new ClassReader(readBytes(resourceName(className)));
        final ClassWriter writer = new ClassWriter(reader, ClassWriter.COMPUTE_MAXS);
        assertTrue(
                new SessionServiceTransformer().transform(reader, writer),
                "the transformer should have applied to " + className);
        return writer.toByteArray();
    }

    /** A loader that owns the given classes and shares everything else with the test. */
    private static ClassLoader loaderFor(Map<String, byte[]> classes) {
        return new ClassLoader(SkinFallbackTest.class.getClassLoader()) {
            @Override
            protected Class<?> loadClass(String name, boolean resolve) throws ClassNotFoundException {
                // Everything else — the profile, the texture classes — has to stay
                // shared with the test, or the results would not be comparable.
                final byte[] own = classes.get(name);
                if (own == null) {
                    return super.loadClass(name, resolve);
                }

                Class<?> loaded = findLoadedClass(name);
                if (loaded == null) {
                    loaded = defineClass(name, own, 0, own.length);
                }
                if (resolve) {
                    resolveClass(loaded);
                }
                return loaded;
            }
        };
    }

    private static String resourceName(String className) {
        return className.replace('.', '/') + ".class";
    }

    private static byte[] readBytes(String resource) throws IOException {
        try (InputStream stream = SkinFallbackTest.class.getClassLoader().getResourceAsStream(resource)) {
            assertNotNull(stream, "missing " + resource);
            final ByteArrayOutputStream out = new ByteArrayOutputStream();
            final byte[] buffer = new byte[8192];
            int read;
            while ((read = stream.read(buffer)) != -1) {
                out.write(buffer, 0, read);
            }
            return out.toByteArray();
        }
    }

    /**
     * Answers every request the same way, and remembers what was asked for.
     *
     * <p>A null body stands for a skin system that has never heard of anyone.
     */
    private static void serve(ServerSocket source, List<String> seen, String payload) {
        final Charset utf8 = Charset.forName("UTF-8");
        while (!source.isClosed()) {
            try (Socket socket = source.accept()) {
                final StringBuilder requestLine = new StringBuilder();
                final InputStream in = socket.getInputStream();
                int c;
                while ((c = in.read()) != -1 && c != '\n') {
                    if (c != '\r') {
                        requestLine.append((char) c);
                    }
                }

                final String[] parts = requestLine.toString().split(" ");
                if (parts.length >= 2) {
                    seen.add(parts[1]);
                }

                final OutputStream out = socket.getOutputStream();
                if (payload == null) {
                    out.write("HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n".getBytes(utf8));
                    out.flush();
                    continue;
                }

                final byte[] body = payload.getBytes(utf8);
                out.write(("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: " + body.length
                                + "\r\nConnection: close\r\n\r\n")
                        .getBytes(utf8));
                out.write(body);
                out.flush();
            } catch (IOException e) {
                // The socket closing is how this thread is asked to stop.
                return;
            }
        }
    }
}
