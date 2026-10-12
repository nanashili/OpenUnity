using System.Reflection;
using OpenUnity.Interop;

namespace UnityEngine;

internal sealed partial class LifecycleDispatcher
{
    private sealed class ComponentReservation
    {
        private readonly LifecycleDispatcher dispatcher;
        private readonly GameObject owner;
        private readonly GameObject? priorOwner;
        private long identity;
        private MonoBehaviour? retainedScript;

        internal ComponentReservation(LifecycleDispatcher dispatcher, GameObject owner, int order)
        {
            this.dispatcher = dispatcher;
            this.owner = owner;
            priorOwner = dispatcher.owners.GetValueOrDefault(owner.Identity);
            long reserved = dispatcher.context.Invoke(runtime =>
                OpenUnityNative.ou_component_reserve(runtime, owner.Identity, 4, order));
            Object.CheckNative(reserved, "component reservation");
            if (reserved == 0)
                throw RuntimeContext.Failure("component reservation", reserved);
            identity = reserved;
        }

        internal MonoBehaviour Construct(Type type, MethodInfo?[] methods)
        {
            _ = checked((int)identity);
            MonoBehaviour script;
            using (new ConstructionScope(new Attachment(dispatcher.context, identity, owner)))
                script = (MonoBehaviour)(Activator.CreateInstance(type)
                    ?? throw new InvalidOperationException("OpenUnity component construction returned null."));
            dispatcher.attachments.Add(identity, script);
            retainedScript = script;
            dispatcher.owners[owner.Identity] = owner;
            dispatcher.callbacks[type] = methods;
            return script;
        }

        internal void Commit()
        {
            RequireSuccess(dispatcher.context.Invoke(runtime =>
                OpenUnityNative.ou_component_commit(runtime, identity)), "component commit");
            identity = 0;
        }

        internal void Abort(Exception primary)
        {
            List<Exception> recoveryFailures = new();
            try
            {
                RemoveRetainedAttachment();
            }
            catch (Exception failure)
            {
                recoveryFailures.Add(failure);
            }
            try
            {
                RestoreOwner();
            }
            catch (Exception failure)
            {
                recoveryFailures.Add(failure);
            }
            try
            {
                RequireSuccess(dispatcher.context.Invoke(runtime =>
                    OpenUnityNative.ou_component_abort(runtime, identity)), "component reservation cleanup");
                identity = 0;
            }
            catch (Exception failure)
            {
                recoveryFailures.Add(failure);
            }
            if (recoveryFailures.Count != 0)
            {
                recoveryFailures.Insert(0, primary);
                AggregateException failure = new("Component attachment and reservation recovery failed.", recoveryFailures);
                dispatcher.failedReservations.Add(this);
                dispatcher.context.RejectFurtherUse(failure);
                throw failure;
            }
        }

        private void RemoveRetainedAttachment()
        {
            if (retainedScript is null)
                return;
            if (!dispatcher.attachments.TryGetValue(identity, out MonoBehaviour? existing) ||
                !ReferenceEquals(existing, retainedScript))
                throw new InvalidOperationException("OpenUnity reserved attachment ownership changed during recovery.");
            if (!dispatcher.attachments.Remove(identity))
                throw new InvalidOperationException("OpenUnity reserved attachment removal failed during recovery.");
            retainedScript = null;
        }

        private void RestoreOwner()
        {
            if (priorOwner is not null)
            {
                dispatcher.owners[owner.Identity] = priorOwner;
                return;
            }
            if (dispatcher.owners.TryGetValue(owner.Identity, out GameObject? existing))
            {
                if (!ReferenceEquals(existing, owner) || !dispatcher.owners.Remove(owner.Identity))
                    throw new InvalidOperationException("OpenUnity reserved owner changed during recovery.");
            }
        }
    }
}
