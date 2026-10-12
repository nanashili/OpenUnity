using OpenUnity.Interop;

namespace UnityEngine;

internal sealed partial class LifecycleDispatcher : INativeComponentReservationHost
{
    private readonly NativeComponentAttachments nativeComponents = new();
    private readonly List<NativeComponentReservation> nativeRecoveryReservations = new();
    private readonly Dictionary<NativeComponentAttachments.Reservation, GameObject?> nativePriorOwners = new();

    // GameObject calls this after entering RuntimeContext.Invoke. Keeping the
    // script branch here preserves the existing callback and construction
    // catalog; native components use the separate canonical-root transaction.
    internal Component AttachComponent(GameObject owner, Type type)
    {
        CheckThread();
        ArgumentNullException.ThrowIfNull(owner);
        ArgumentNullException.ThrowIfNull(type);

        if (typeof(MonoBehaviour).IsAssignableFrom(type))
            return Attach(owner, type);

        NativeComponentBinding binding = ResolveNativeBinding(
            type,
            NativeComponentBindingOperation.Create);
        if (!binding.CanCreate || binding.Factory is null)
            throw new NotSupportedException($"OpenUnity cannot attach {type.FullName}.");

        return AttachNativeComponent(owner, binding);
    }

    private Component AttachNativeComponent(
        GameObject owner,
        NativeComponentBinding binding)
    {
        NativeComponentReservation transaction = NativeComponentReservation.Begin(
            context,
            this,
            binding,
            owner);
        Component wrapper;

        try
        {
            transaction.Prepare();
            wrapper = transaction.Wrapper;
            transaction.Commit();
        }
        catch (Exception failure)
        {
            transaction.Recover(failure);
            throw;
        }

        PumpMutations();
        return wrapper;
    }

    // Binding resolution is implemented by the Core binding candidate. This
    // method is the single admission seam used by attachment and full-query
    // paths, so Type metadata is never interpreted in two places.
    private static NativeComponentBinding ResolveNativeBinding(
        Type type,
        NativeComponentBindingOperation operation) =>
        NativeComponentBindingResolution.Resolve(type, operation);

    NativeComponentAttachments.Reservation INativeComponentReservationHost.PrepareReservation(GameObject owner)
    {
        ArgumentNullException.ThrowIfNull(owner);
        if (owners.TryGetValue(owner.Identity, out GameObject? existingOwner) &&
            !ReferenceEquals(existingOwner, owner))
            throw new InvalidOperationException(
                "A native owner identity is bound to a different managed owner.");

        nativeRecoveryReservations.EnsureCapacity(nativeRecoveryReservations.Count + 1);
        nativePriorOwners.EnsureCapacity(nativePriorOwners.Count + 1);
        owners.EnsureCapacity(owners.Count + (owners.ContainsKey(owner.Identity) ? 0 : 1));

        NativeComponentAttachments.Reservation preparation =
            nativeComponents.PrepareReservation(owner);
        try
        {
            nativePriorOwners.Add(preparation, owners.GetValueOrDefault(owner.Identity));
            return preparation;
        }
        catch (Exception failure)
        {
            try
            {
                nativeComponents.CancelPreparation(preparation);
            }
            catch (Exception cleanup)
            {
                throw new AggregateException(
                    "Native owner preparation and rollback failed.",
                    failure,
                    cleanup);
            }

            throw;
        }
    }

    NativeComponentAttachments.Reservation INativeComponentReservationHost.RegisterPending(
        long identity,
        NativeComponentAttachments.Reservation preparation) =>
        nativeComponents.RegisterPending(identity, preparation);

    void INativeComponentReservationHost.CancelPreparation(
        NativeComponentAttachments.Reservation preparation) =>
        nativeComponents.CancelPreparation(preparation);

    void INativeComponentReservationHost.PrepareCommit(
        NativeComponentAttachments.Reservation reservation,
        Component wrapper)
    {
        nativeComponents.PrepareCommit(reservation, wrapper);
        owners[reservation.Owner.Identity] = reservation.Owner;
    }

    void INativeComponentReservationHost.CommitPrepared(
        NativeComponentAttachments.Reservation reservation)
    {
        nativeComponents.CommitPrepared(reservation);
        if (!nativePriorOwners.Remove(reservation))
            throw new InvalidOperationException("The native component owner recovery snapshot could not be released.");
    }

    void INativeComponentReservationHost.CompleteAbort(
        NativeComponentAttachments.Reservation reservation) =>
        nativeComponents.CompleteAbort(reservation);

    void INativeComponentReservationHost.RestoreOwner(
        NativeComponentAttachments.Reservation reservation)
    {
        if (!nativePriorOwners.TryGetValue(reservation, out GameObject? priorOwner))
            throw new InvalidOperationException("The native component owner recovery snapshot is unavailable.");

        if (priorOwner is not null)
        {
            if (owners.TryGetValue(reservation.Owner.Identity, out GameObject? existing) &&
                !ReferenceEquals(existing, reservation.Owner) &&
                !ReferenceEquals(existing, priorOwner))
                throw new InvalidOperationException("The native component owner changed during recovery.");

            owners[reservation.Owner.Identity] = priorOwner;
            return;
        }

        if (owners.TryGetValue(reservation.Owner.Identity, out GameObject? currentOwner) &&
            (!ReferenceEquals(currentOwner, reservation.Owner) || !owners.Remove(reservation.Owner.Identity)))
            throw new InvalidOperationException("The native component owner changed during recovery.");
    }

    void INativeComponentReservationHost.RetainRecovery(NativeComponentReservation reservation)
    {
        if (!nativeRecoveryReservations.Contains(reservation))
            nativeRecoveryReservations.Add(reservation);
    }

    void INativeComponentReservationHost.ReleaseRecovery(NativeComponentReservation reservation)
    {
        nativeRecoveryReservations.Remove(reservation);
        if (reservation.PreparedAttachment is NativeComponentAttachments.Reservation attachment)
            nativePriorOwners.Remove(attachment);
    }
}
