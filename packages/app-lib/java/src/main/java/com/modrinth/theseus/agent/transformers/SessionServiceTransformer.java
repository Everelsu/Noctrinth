package com.modrinth.theseus.agent.transformers;

import java.util.ListIterator;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.Type;
import org.objectweb.asm.tree.AbstractInsnNode;
import org.objectweb.asm.tree.ClassNode;
import org.objectweb.asm.tree.MethodInsnNode;
import org.objectweb.asm.tree.MethodNode;
import org.objectweb.asm.tree.TypeInsnNode;
import org.objectweb.asm.tree.VarInsnNode;

/**
 * Gives the game somewhere to get a skin from when the server sent none.
 *
 * <p>Offline-mode servers hand out profiles without a {@code textures} property, and the client
 * never asks anyone about it — it renders whatever the profile carried, which is why everyone is
 * Steve there. This wraps what authlib's {@code getTextures} is about to return, so a profile that
 * came back bare gets one more chance at a skin, looked up by name.
 *
 * <p>The wrap is deliberately at the very end of the method: whatever authlib decided still
 * happens, and a profile that already has textures returns untouched.
 */
public final class SessionServiceTransformer extends ClassNodeTransformer {
    public static final String TARGET_CLASS = "com/mojang/authlib/yggdrasil/YggdrasilMinecraftSessionService";

    private static final String METHOD_NAME = "getTextures";
    private static final String GAME_PROFILE = "Lcom/mojang/authlib/GameProfile;";
    private static final String HOOKS_CLASS = "com/modrinth/theseus/agent/skins/SkinHooks";
    private static final String HOOK_NAME = "fillTextures";
    private static final String HOOK_DESC = "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;";

    @Override
    protected boolean transform(ClassNode classNode) {
        boolean transformed = false;

        for (final MethodNode method : classNode.methods) {
            if (!METHOD_NAME.equals(method.name) || method.name.indexOf('$') != -1) {
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

            // The profile is the first argument, after `this` on an instance method.
            final int profileSlot = (method.access & Opcodes.ACC_STATIC) != 0 ? 0 : 1;
            transformed |= wrapReturns(method, profileSlot, returnType);
        }

        return transformed;
    }

    private static boolean wrapReturns(MethodNode method, int profileSlot, Type returnType) {
        boolean wrapped = false;

        final ListIterator<AbstractInsnNode> it = method.instructions.iterator();
        while (it.hasNext()) {
            final AbstractInsnNode insn = it.next();
            if (insn.getOpcode() != Opcodes.ARETURN) {
                continue;
            }

            // The value being returned is on the stack; hand it to the hook along
            // with the profile it belongs to, and return whatever comes back.
            it.previous();
            it.add(new VarInsnNode(Opcodes.ALOAD, profileSlot));
            it.add(new MethodInsnNode(Opcodes.INVOKESTATIC, HOOKS_CLASS, HOOK_NAME, HOOK_DESC, false));
            it.add(new TypeInsnNode(Opcodes.CHECKCAST, returnType.getInternalName()));
            it.next();
            wrapped = true;
        }

        return wrapped;
    }
}
