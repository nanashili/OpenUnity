using System.Reflection;
using OpenUnity.Interop;

namespace UnityEngine;

// Native state selects every event. This adapter only binds constructors,
// retains canonical wrappers and invokes the returned callback snapshots.
internal sealed class LifecycleDispatcher
{
    private static readonly string[] EventNames =
        ["Awake", "OnEnable", "Start", "Update", "LateUpdate", "OnDisable", "OnDestroy"];
    [ThreadStatic] private static ConstructionScope? construction;
    private readonly RuntimeContext context;
    private readonly int ownerThread = Environment.CurrentManagedThreadId;
    private readonly Dictionary<long, MonoBehaviour> attachments = new();
    private readonly Dictionary<long, GameObject> owners = new();
    private readonly Dictionary<Type, MethodInfo?[]> callbacks = new();
    private int pumpDepth;
    private bool timed;
    private readonly Dictionary<Type, MethodInfo?> fixedCallbacks = new();

    internal LifecycleDispatcher(RuntimeContext context)
    {
        uint abi = OpenUnityNative.ou_lifecycle_abi_version();
        if (abi != OpenUnityNative.LifecycleAbiVersion)
            throw new InvalidOperationException($"OpenUnity lifecycle ABI 1.0 is required; received 0x{abi:X8}.");
        this.context = context;
    }

    internal void CheckThread()
    {
        RuntimeContext.RequireEngineAccess();
        if (Environment.CurrentManagedThreadId != ownerThread)
            throw new NotSupportedException("OpenUnity lifecycle operations must run on their context's owner thread.");
    }

    internal static void ValidateScriptType(Type type)
    {
        ArgumentNullException.ThrowIfNull(type);
        if (!type.IsClass || type.IsAbstract || type.ContainsGenericParameters ||
            type == typeof(MonoBehaviour) || !typeof(MonoBehaviour).IsAssignableFrom(type) ||
            type.GetConstructor(Type.EmptyTypes) is null)
            throw new NotSupportedException("OpenUnity attachment requires a concrete custom MonoBehaviour with a public parameterless constructor.");
    }

    internal MonoBehaviour Attach(GameObject owner, Type type)
    {
        CheckThread();
        ValidateScriptType(type);
        MethodInfo?[] methods = ResolveCallbacks(type);
        if (timed)
            PreflightTimedType(type);
        int order = type.GetCustomAttribute<DefaultExecutionOrder>(inherit: false)?.order ?? 0;
        long identity = context.Invoke(runtime => OpenUnityNative.ou_component_reserve(runtime, owner.Identity, 4, order));
        Object.CheckNative(identity, "component reservation");
        if (identity == 0)
            throw RuntimeContext.Failure("component reservation", identity);
        bool committed = false;
        bool retainedOwner = owners.ContainsKey(owner.Identity);
        try
        {
            _ = checked((int)identity);
            MonoBehaviour script;
            using (new ConstructionScope(new Attachment(context, identity, owner)))
                script = (MonoBehaviour)(Activator.CreateInstance(type)
                    ?? throw new InvalidOperationException("OpenUnity component construction returned null."));
            attachments.Add(identity, script);
            owners[owner.Identity] = owner;
            callbacks[type] = methods;
            RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_component_commit(runtime, identity)), "component commit");
            committed = true;
            PumpMutations();
            return script;
        }
        catch
        {
            if (!committed)
            {
                attachments.Remove(identity);
                if (!retainedOwner)
                    owners.Remove(owner.Identity);
                RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_component_abort(runtime, identity)), "component reservation cleanup");
            }
            throw;
        }
    }

    internal void PreflightSceneType(Type type)
    {
        CheckThread();
        ValidateScriptType(type);
        callbacks[type] = ResolveCallbacks(type);
    }

    internal void PreflightTimedType(Type type)
    {
        CheckThread();
        if (!fixedCallbacks.ContainsKey(type))
            fixedCallbacks.Add(type, ResolveCallback(type, "FixedUpdate"));
    }

    internal MonoBehaviour ConstructSceneScript(GameObject owner, long identity, Type type)
    {
        CheckThread();
        MonoBehaviour script;
        using (new ConstructionScope(new Attachment(context, identity, owner)))
        using (RuntimeContext.SceneConstructorScope())
            script = (MonoBehaviour)(Activator.CreateInstance(type)
                ?? throw new InvalidOperationException("OpenUnity scene construction returned null."));
        attachments.Add(identity, script);
        return script;
    }

    internal void ReleaseSceneBindings(IEnumerable<long> scripts, IEnumerable<GameObject> sceneOwners)
    {
        CheckThread();
        HashSet<GameObject> releasedOwners = new(sceneOwners, ReferenceEqualityComparer.Instance);
        foreach (long identity in scripts)
            attachments.Remove(identity);
        foreach (long identity in attachments.Where(item => releasedOwners.Contains(item.Value.AttachedOwner))
                     .Select(item => item.Key).ToArray())
            attachments.Remove(identity);
        foreach (GameObject owner in releasedOwners)
        {
            if (owners.TryGetValue(owner.Identity, out GameObject? existing) && ReferenceEquals(existing, owner))
                owners.Remove(owner.Identity);
            GameObject.ReleaseSceneWrapper(owner);
        }
    }

    internal Component[] Components(GameObject owner, Type type)
    {
        CheckThread();
        ArgumentNullException.ThrowIfNull(type);
        if (type != typeof(Transform) && type != typeof(Component) &&
            type != typeof(Behaviour) && type != typeof(MonoBehaviour) &&
            !(type.IsClass && typeof(MonoBehaviour).IsAssignableFrom(type)))
            throw new NotSupportedException($"OpenUnity component queries do not support {type.FullName}.");
        List<Component> found = new();
        context.Invoke(runtime =>
        {
            int count = OpenUnityNative.ou_gameobject_component_count(runtime, owner.Identity);
            Object.CheckNative(count, "component count query");
            for (int index = 0; index < count; index++)
            {
                long identity = OpenUnityNative.ou_gameobject_component_at(runtime, owner.Identity, index);
                Object.CheckNative(identity, "component identity query");
                Component component = index == 0 ? owner.transform : attachments.TryGetValue(identity, out MonoBehaviour? script)
                    ? script : throw new InvalidOperationException("OpenUnity returned a component without a canonical managed attachment.");
                if (type.IsAssignableFrom(component.GetType()))
                    found.Add(component);
            }
            return 0;
        });
        return found.ToArray();
    }

    internal bool ReadBoolean(Func<long, int> operation, string name)
    {
        CheckThread();
        int result = context.Invoke(operation);
        Object.CheckNative(result, name);
        return result switch
        {
            0 => false,
            1 => true,
            _ => throw RuntimeContext.Failure(name, result)
        };
    }

    internal void Mutate(Func<long, int> operation, string name)
    {
        CheckThread();
        RequireSuccess(context.Invoke(operation), name);
        PumpMutations();
    }

    internal int FrameCount
    {
        get
        {
            CheckThread();
            int count = context.Invoke(OpenUnityNative.ou_lifecycle_frame_count);
            Object.CheckNative(count, "frame count query");
            return count;
        }
    }

    internal int Tick()
    {
        CheckThread();
        if (pumpDepth != 0)
            throw RuntimeContext.Failure("reentrant frame begin", -13);
        RequireSuccess(context.Invoke(OpenUnityNative.ou_lifecycle_begin_frame), "frame begin");
        for (int phase = 1; phase <= 5; phase++)
        {
            RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_lifecycle_begin_phase(runtime, phase)), "frame phase begin");
            Pump(1);
        }
        return FrameCount;
    }

    private static void RequireTimeCapability()
    {
        uint version = OpenUnityNative.ou_time_abi_version();
        if (version != OpenUnityNative.TimeAbiVersion)
            throw new NotSupportedException($"OpenUnity Time ABI 1.0 is required; received 0x{version:X8}.");
    }

    internal (double[] Values, bool InFixedTimeStep) ReadTime()
    {
        CheckThread();
        RequireTimeCapability();
        double[] values = new double[6];
        int frame = 0;
        int fixedContext = 0;
        RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_time_snapshot(
            runtime, values, values.Length, out frame, out fixedContext)), "Time snapshot");
        if (fixedContext is not (0 or 1))
            throw new InvalidOperationException("OpenUnity returned an invalid Time context flag.");
        return (values, fixedContext == 1);
    }

    internal void SetTime(int property, float value)
    {
        CheckThread();
        RequireTimeCapability();
        RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_time_set(runtime, property, value)), "Time setting");
    }

    internal void ConfigureTime(int source, double step)
    {
        CheckThread();
        RequireTimeCapability();
        RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_time_configure(runtime, source, step)), "Time configuration");
        timed = true;
    }

    internal int TickTimed()
    {
        CheckThread();
        if (pumpDepth != 0)
            throw RuntimeContext.Failure("reentrant frame dispatch", -13);
        RequireSuccess(context.Invoke(OpenUnityNative.ou_time_begin_frame), "timed frame begin");
        BeginTimedPhase(1);
        while (true)
        {
            int status = context.Invoke(OpenUnityNative.ou_time_begin_fixed);
            if (status == 0)
                break;
            RequireSuccess(status == 1 ? 0 : status, "fixed iteration begin");
            Pump(2);
        }
        for (int phase = 2; phase <= 5; phase++)
            BeginTimedPhase(phase);
        return FrameCount;
    }

    private void BeginTimedPhase(int phase)
    {
        RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_lifecycle_begin_phase(runtime, phase)),
            "timed frame phase begin");
        Pump(2);
    }

    internal void FlushDestroy(bool timed = false)
    {
        CheckThread();
        if (pumpDepth != 0)
            throw RuntimeContext.Failure("reentrant destruction boundary", -13);
        RequireSuccess(context.Invoke(OpenUnityNative.ou_lifecycle_flush_destroy), "destruction boundary");
        Pump(timed ? 2 : 1);
    }

    internal void PumpMutations() => Pump(0);

    private void Pump(int mode)
    {
        CheckThread();
        pumpDepth++;
        Exception? failure = null;
        try
        {
            DrainCallbacks(mode);
        }
        catch (Exception error)
        {
            failure = error;
        }
        pumpDepth--;
        if (pumpDepth == 0)
        {
            try
            {
                ReleaseDeadAttachments();
            }
            catch (Exception cleanup)
            {
                failure = failure is null ? cleanup :
                    new AggregateException("Lifecycle dispatch and attachment cleanup failed.", failure, cleanup);
            }
        }
        if (failure is not null)
            System.Runtime.ExceptionServices.ExceptionDispatchInfo.Capture(failure).Throw();
    }

    private void DrainCallbacks(int mode)
    {
        while (true)
        {
            long token = 0;
            long identity = 0;
            int lifecycleEvent = 0;
            int status = context.Invoke(runtime => mode == 2
                ? OpenUnityNative.ou_time_next(runtime, out token, out identity, out lifecycleEvent)
                : OpenUnityNative.ou_lifecycle_next(runtime, mode, out token, out identity, out lifecycleEvent));
            if (status == 0)
                return;
            RequireSuccess(status == 1 ? 0 : status, "lifecycle event poll");
            DeliverCallback(mode, token, identity, lifecycleEvent);
        }
    }

    private void DeliverCallback(int mode, long token, long identity, int lifecycleEvent)
    {
        try
        {
            int maximumEvent = mode == 2 ? 8 : EventNames.Length;
            if (lifecycleEvent < 1 || lifecycleEvent > maximumEvent ||
                !attachments.TryGetValue(identity, out MonoBehaviour? script))
                throw new InvalidOperationException("OpenUnity returned an invalid managed lifecycle snapshot.");
            MethodInfo? callback = lifecycleEvent == 8
                ? fixedCallbacks[script.GetType()]
                : callbacks[script.GetType()][lifecycleEvent - 1];
            callback?.Invoke(script, null);
        }
        catch (Exception error)
        {
            Exception failure = error is TargetInvocationException { InnerException: not null } invoked
                ? invoked.InnerException : error;
            try
            {
                int status = context.Invoke(runtime => OpenUnityNative.ou_lifecycle_ack(runtime, token, -6));
                if (status != 0 && status != -14)
                    throw RuntimeContext.Failure("failed callback acknowledgement", status);
            }
            catch (Exception cleanup)
            {
                throw new AggregateException("Gameplay callback and token cleanup failed.", failure, cleanup);
            }
            throw new InvalidOperationException("OpenUnity gameplay callback failed; dispatch is faulted.", failure);
        }
        RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_lifecycle_ack(runtime, token, 0)),
            "lifecycle event acknowledgement");
    }

    internal void RetainOwner(GameObject owner)
    {
        CheckThread();
        owners[owner.Identity] = owner;
    }

    internal GameObject? FindOwner(long identity)
    {
        CheckThread();
        return owners.GetValueOrDefault(identity);
    }

    private void ReleaseDeadAttachments()
    {
        context.Invoke(runtime =>
        {
            foreach (long identity in attachments.Keys.ToArray())
                if (!IsAlive(runtime, identity))
                    attachments.Remove(identity);
            foreach (long identity in owners.Keys.ToArray())
                if (!IsAlive(runtime, identity))
                    owners.Remove(identity);
            return 0;
        });
    }

    private static bool IsAlive(long runtime, long identity) =>
        OpenUnityNative.ou_object_is_alive(runtime, identity) switch
        {
            0 => false,
            1 => true,
            int status => throw RuntimeContext.Failure("attachment liveness query", status)
        };

    private MethodInfo?[] ResolveCallbacks(Type type)
    {
        if (callbacks.TryGetValue(type, out MethodInfo?[]? existing))
            return existing;
        MethodInfo?[] methods = new MethodInfo?[EventNames.Length];
        for (int index = 0; index < EventNames.Length; index++)
            methods[index] = ResolveCallback(type, EventNames[index]);
        return methods;
    }

    private static MethodInfo? ResolveCallback(Type type, string name)
    {
        for (Type? current = type; current != typeof(MonoBehaviour) && current is not null; current = current.BaseType)
        {
            MethodInfo[] candidates = current.GetMethods(BindingFlags.Instance | BindingFlags.Static |
                BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.DeclaredOnly)
                .Where(method => method.Name == name).ToArray();
            if (candidates.Length == 0)
                continue;
            if (candidates.Length != 1 || candidates[0].IsStatic || candidates[0].IsGenericMethod ||
                candidates[0].ReturnType != typeof(void) || candidates[0].GetParameters().Length != 0)
                throw new NotSupportedException(
                    $"OpenUnity lifecycle method {name} must be one instance void parameterless method.");
            return candidates[0];
        }
        return null;
    }

    private static void RequireSuccess(int status, string operation)
    {
        Object.CheckNative(status, operation);
        if (status != 0)
            throw RuntimeContext.Failure(operation, status);
    }

    internal readonly record struct Attachment(RuntimeContext Context, long Identity, GameObject Owner);

    internal static Attachment ConsumeAttachment()
    {
        ConstructionScope scope = construction ?? throw new NotSupportedException("OpenUnity components must be constructed by GameObject.AddComponent.");
        if (scope.Consumed)
            throw new NotSupportedException("OpenUnity attachment construction may be consumed only once.");
        scope.Consumed = true;
        return scope.Attachment;
    }

    private sealed class ConstructionScope : IDisposable
    {
        private readonly ConstructionScope? previous = construction;
        internal Attachment Attachment { get; }
        internal bool Consumed { get; set; }

        internal ConstructionScope(Attachment attachment)
        {
            Attachment = attachment;
            construction = this;
        }

        public void Dispose() => construction = previous;
    }
}
