using System;
using System.IO;
using System.Runtime.InteropServices;

namespace OpenUnity.Interop;

/// <summary>Exact managed declarations for OpenUnity ABI v1.</summary>
internal static partial class OpenUnityNative
{
    internal const uint AbiVersion = 0x0001_0000;
    internal const uint ObjectNameAbiVersion = 0x0001_0000;
    internal const uint TransformAbiVersion = 0x0001_0000;
    internal const uint TransformHierarchyAbiVersion = 0x0001_0000;
    internal const uint LifecycleAbiVersion = 0x0001_0000;
    internal const uint SceneAbiVersion = 0x0001_0000;
    internal const uint TimeAbiVersion = 0x0001_0000;
    internal const uint PhysicsGravityAbiVersion = 0x0001_0000;

    static OpenUnityNative()
    {
        string? configuredPath = Environment.GetEnvironmentVariable("OPENUNITY_NATIVE_LIBRARY");
        if (configuredPath is null)
            return;
        if (!Path.IsPathRooted(configuredPath))
            throw new ArgumentException("OPENUNITY_NATIVE_LIBRARY must be an absolute path", nameof(configuredPath));
        if (!File.Exists(configuredPath))
            throw new DllNotFoundException($"OPENUNITY_NATIVE_LIBRARY does not exist: {configuredPath}");

        NativeLibrary.SetDllImportResolver(typeof(OpenUnityNative).Assembly,
            (libraryName, _, _) => libraryName == "openunity_engine"
                ? NativeLibrary.Load(configuredPath)
                : IntPtr.Zero);
    }

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern uint ou_abi_version();

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern long ou_runtime_create();

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_runtime_destroy(long runtime);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern long ou_object_create(long runtime);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_object_is_alive(long runtime, long obj);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_object_destroy(long runtime, long obj);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern uint ou_object_name_abi_version();

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_object_name_len(long runtime, long obj);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_object_name_read(long runtime, long obj,
        [Out, MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 3)] byte[]? output, int capacity);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_object_name_set(long runtime, long obj,
        [In, MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 3)] byte[]? input, int length);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern uint ou_transform_abi_version();

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern long ou_gameobject_create(long runtime,
        [In, MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 2)] byte[]? name, int length);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_object_kind(long runtime, long obj);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern long ou_gameobject_transform(long runtime, long gameObject);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern long ou_component_gameobject(long runtime, long component);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_transform_read(long runtime, long transform, int property,
        [Out, MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 4)] float[] output, int count);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_transform_write(long runtime, long transform, int property,
        [In, MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 4)] float[] input, int count);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern long ou_transform_parent(long runtime, long transform);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_transform_child_count(long runtime, long transform);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern long ou_transform_child(long runtime, long transform, int index);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_transform_set_parent(long runtime, long transform, long parent, int worldPositionStays);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern uint ou_transform_hierarchy_abi_version();

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_transform_set_sibling_index(long runtime, long transform, int index);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_transform_point(long runtime, long transform,
        [In, MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 4)] float[] input,
        [Out, MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 4)] float[] output, int count, int inverse);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern uint ou_lifecycle_abi_version();
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern long ou_component_reserve(long runtime, long owner, int kind, int executionOrder);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_component_commit(long runtime, long component);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_component_abort(long runtime, long component);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_gameobject_component_count(long runtime, long owner);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern long ou_gameobject_component_at(long runtime, long owner, int index);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_gameobject_active(long runtime, long owner, int hierarchy);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_gameobject_set_active(long runtime, long owner, int value);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_behaviour_enabled(long runtime, long component);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_behaviour_set_enabled(long runtime, long component, int value);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_behaviour_active(long runtime, long component);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_lifecycle_frame_count(long runtime);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_lifecycle_begin_frame(long runtime);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_lifecycle_begin_phase(long runtime, int phase);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_lifecycle_flush_destroy(long runtime);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_lifecycle_next(long runtime, int mode, out long token, out long component, out int lifecycleEvent);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_lifecycle_ack(long runtime, long token, int callbackStatus);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern uint ou_time_abi_version();
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_time_configure(long runtime, int source, double step);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_time_set(long runtime, int property, float value);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_time_snapshot(long runtime, [Out] double[] values, int count,
        out int frameCount, out int inFixedTimeStep);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_time_begin_frame(long runtime);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_time_begin_fixed(long runtime);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_time_next(long runtime, int mode, out long token, out long component, out int lifecycleEvent);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern uint ou_physics_gravity_abi_version();

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_physics_gravity_read(long runtime,
        [Out, MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 2)] float[] output, int count);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_physics_gravity_write(long runtime,
        [In, MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 2)] float[] input, int count);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_object_destroy_deferred(long runtime, long obj);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern uint ou_scene_abi_version();
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_transform_sibling_index(long runtime, long transform);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern long ou_scene_stage(long runtime,
        [In, MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 2)] byte[] plan, int byteCount);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern long ou_scene_object(long runtime, long scene, long fileId);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_scene_publish(long runtime, long scene);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_scene_abort(long runtime, long scene);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_scene_destroy(long runtime, long scene);
    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_scene_release(long runtime, long scene);
}
