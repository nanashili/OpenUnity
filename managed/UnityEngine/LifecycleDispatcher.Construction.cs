using System.Reflection;

namespace UnityEngine;

internal sealed partial class LifecycleDispatcher
{
    internal MonoBehaviour Attach(GameObject owner, Type type)
    {
        CheckThread();
        ValidateScriptType(type);
        MethodInfo?[] methods = ResolveCallbacks(type);
        if (timed)
            PreflightTimedType(type);
        int order = type.GetCustomAttribute<DefaultExecutionOrder>(inherit: false)?.order ?? 0;
        ComponentReservation reservation = new(this, owner, order);
        MonoBehaviour script;
        try
        {
            script = reservation.Construct(type, methods);
            reservation.Commit();
        }
        catch (Exception failure)
        {
            reservation.Abort(failure);
            throw;
        }
        PumpMutations();
        return script;
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
