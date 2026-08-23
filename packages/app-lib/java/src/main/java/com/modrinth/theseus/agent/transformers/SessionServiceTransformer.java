package com.modrinth.theseus.agent.transformers;

import java.util.ArrayList;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.Type;
import org.objectweb.asm.tree.ClassNode;
import org.objectweb.asm.tree.FrameNode;
import org.objectweb.asm.tree.InsnList;
import org.objectweb.asm.tree.InsnNode;
import org.objectweb.asm.tree.LabelNode;
import org.objectweb.asm.tree.LdcInsnNode;
import org.objectweb.asm.tree.MethodInsnNode;
import org.objectweb.asm.tree.MethodNode;
import org.objectweb.asm.tree.TryCatchBlockNode;
import org.objectweb.asm.tree.TypeInsnNode;
import org.objectweb.asm.tree.VarInsnNode;

/**
 * Gives the game somewhere to get a skin from when the server sent none it can use.
 *
 * <p>Offline-mode servers hand out profiles without a {@code textures} property, and the client
 * never asks anyone about it — it renders whatever the profile carried, which is why everyone is
 * Steve there. Worse, a profile that <em>does</em> carry textures signed by an account system this
 * client does not have the key for — an Ely.by player seen by a licensed one, or the other way
 * about — makes authlib throw rather than return, and the game treats that as no skin at all.
 *
 * <p>So {@code getTextures} is renamed aside and a wrapper of the same name put in its place: what
 * it returns still goes through {@link com.modrinth.theseus.agent.skins.SkinHooks}, and what it
 * throws does too, with the throw passed on untouched if the lookup has nothing better to offer.
 * Wrapping rather than rewriting the body also keeps the original's stack map frames out of it —
 * the only frame here is the one on the handler, and this writes it itself.
 *
 * <p>Both homes {@code getTextures} has had are covered. It was a method on the Yggdrasil service
 * until 1.20.2, and a default method on the {@code MinecraftSessionService} interface after it.
 */
public final class SessionServiceTransformer extends ClassNodeTransformer {
    /** Where {@code getTextures} lived up to 1.20.1. */
    public static final String YGGDRASIL_CLASS = "com/mojang/authlib/yggdrasil/YggdrasilMinecraftSessionService";

    /** Where it lives from 1.20.2 on, as a default method. */
    public static final String SESSION_SERVICE_CLASS = "com/mojang/authlib/minecraft/MinecraftSessionService";

    private static final String METHOD_NAME = "getTextures";

    /** What the original is renamed to. The {@code $} keeps it out of a second pass. */
    private static final String WRAPPED_NAME = METHOD_NAME + "$noctrinthOriginal";

    private static final String GAME_PROFILE = "Lcom/mojang/authlib/GameProfile;";
    private static final String THROWABLE = "java/lang/Throwable";
    private static final String HOOKS_CLASS = "com/modrinth/theseus/agent/skins/SkinHooks";

    private static final String FILL_HOOK = "fillTextures";
    private static final String FILL_DESC = "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;";
    private static final String RECOVER_HOOK = "recoverTextures";
    private static final String RECOVER_DESC =
            "(Ljava/lang/Throwable;Ljava/lang/Object;Ljava/lang/Class;)Ljava/lang/Object;";

    @Override
    protected boolean transform(ClassNode classNode) {
        boolean transformed = false;

        for (final MethodNode method : new ArrayList<>(classNode.methods)) {
            if (!METHOD_NAME.equals(method.name)) {
                continue;
            }

            // An abstract declaration has no body to wrap, and whoever implements
            // it is where the wrapping has to happen instead.
            if ((method.access & (Opcodes.ACC_ABSTRACT | Opcodes.ACC_STATIC)) != 0) {
                continue;
            }

            final Type[] arguments = Type.getArgumentTypes(method.desc);
            if (arguments.length == 0 || !GAME_PROFILE.equals(arguments[0].getDescriptor())) {
                continue;
            }

            final Type returnType = Type.getReturnType(method.desc);
            if (returnType.getSort() != Type.OBJECT) {
                continue;
            }

            // Already ours: this is the wrapper, from a pass that has been here
            // before. Wrapping it again would only add a hop.
            if (declares(classNode, WRAPPED_NAME, method.desc)) {
                continue;
            }

            wrap(classNode, method, arguments, returnType);
            transformed = true;
        }

        return transformed;
    }

    private static boolean declares(ClassNode classNode, String name, String desc) {
        for (final MethodNode method : classNode.methods) {
            if (name.equals(method.name) && desc.equals(method.desc)) {
                return true;
            }
        }

        return false;
    }

    private void wrap(ClassNode classNode, MethodNode original, Type[] arguments, Type returnType) {
        final boolean isInterface = (classNode.access & Opcodes.ACC_INTERFACE) != 0;
        final MethodNode wrapper = new MethodNode(
                Opcodes.ASM9,
                original.access,
                original.name,
                original.desc,
                original.signature,
                original.exceptions.toArray(new String[0]));

        original.name = WRAPPED_NAME;
        original.access |= Opcodes.ACC_SYNTHETIC;

        final LabelNode start = new LabelNode();
        final LabelNode end = new LabelNode();
        final LabelNode handler = new LabelNode();
        wrapper.tryCatchBlocks.add(new TryCatchBlockNode(start, end, handler, THROWABLE));

        final InsnList code = wrapper.instructions;
        code.add(start);

        // Call what used to be here, with the arguments exactly as they came in.
        code.add(new VarInsnNode(Opcodes.ALOAD, 0));
        int slot = 1;
        for (final Type argument : arguments) {
            code.add(new VarInsnNode(argument.getOpcode(Opcodes.ILOAD), slot));
            slot += argument.getSize();
        }
        code.add(new MethodInsnNode(
                isInterface ? Opcodes.INVOKEINTERFACE : Opcodes.INVOKEVIRTUAL,
                classNode.name,
                WRAPPED_NAME,
                original.desc,
                isInterface));

        // What it answered, together with the profile it answered for.
        code.add(new VarInsnNode(Opcodes.ALOAD, 1));
        code.add(new MethodInsnNode(Opcodes.INVOKESTATIC, HOOKS_CLASS, FILL_HOOK, FILL_DESC, false));
        code.add(new TypeInsnNode(Opcodes.CHECKCAST, returnType.getInternalName()));
        code.add(end);
        code.add(new InsnNode(Opcodes.ARETURN));

        code.add(handler);
        if (needsStackMap(classNode)) {
            // Nothing in the wrapper writes to a local, so the handler sees exactly
            // what the method was called with.
            code.add(new FrameNode(
                    Opcodes.F_FULL, slot, entryLocals(classNode, arguments, slot), 1, new Object[] {THROWABLE}));
        }
        code.add(new VarInsnNode(Opcodes.ALOAD, 1));
        // The shape to build, for a failure that left us nothing to copy it from.
        code.add(new LdcInsnNode(returnType));
        code.add(new MethodInsnNode(Opcodes.INVOKESTATIC, HOOKS_CLASS, RECOVER_HOOK, RECOVER_DESC, false));
        code.add(new TypeInsnNode(Opcodes.CHECKCAST, returnType.getInternalName()));
        code.add(new InsnNode(Opcodes.ARETURN));

        // Recomputed on the way out; these only have to be legal.
        wrapper.maxLocals = slot;
        wrapper.maxStack = slot + 2;

        classNode.methods.add(wrapper);
    }

    /** The frame the wrapper starts with: {@code this}, then the arguments. */
    private static Object[] entryLocals(ClassNode classNode, Type[] arguments, int slots) {
        final Object[] locals = new Object[slots];
        locals[0] = classNode.name;

        int slot = 1;
        for (final Type argument : arguments) {
            locals[slot] = frameType(argument);
            // A long or a double takes two slots, and only the first is named.
            for (int i = 1; i < argument.getSize(); i++) {
                locals[slot + i] = Opcodes.TOP;
            }
            slot += argument.getSize();
        }

        return locals;
    }

    private static Object frameType(Type type) {
        switch (type.getSort()) {
            case Type.BOOLEAN:
            case Type.BYTE:
            case Type.CHAR:
            case Type.SHORT:
            case Type.INT:
                return Opcodes.INTEGER;
            case Type.FLOAT:
                return Opcodes.FLOAT;
            case Type.LONG:
                return Opcodes.LONG;
            case Type.DOUBLE:
                return Opcodes.DOUBLE;
            case Type.ARRAY:
                return type.getDescriptor();
            default:
                return type.getInternalName();
        }
    }
}
