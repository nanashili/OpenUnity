using System.Collections.Generic;

namespace UnityEngine;

// The dispatcher owns one store for one RuntimeContext. Native component
// wrappers stay strongly rooted here until the native object is retired.
internal sealed partial class NativeComponentAttachments
{
    private readonly Dictionary<long, Component> committed = new();
    private readonly Dictionary<long, Reservation> pending = new();
    private readonly Dictionary<long, Reservation> pendingOwners = new();
    private readonly Dictionary<long, OwnerRoots> rootsByOwner = new();

    // Call before native reserve. This validates owner identity and reserves
    // every table/root capacity needed by the later publication transfer.
    internal Reservation PrepareReservation(GameObject owner)
    {
        ArgumentNullException.ThrowIfNull(owner);

        OwnerRoots? existingRoots = rootsByOwner.GetValueOrDefault(owner.Identity);
        if (existingRoots is not null && !ReferenceEquals(existingRoots.Owner, owner))
            throw new InvalidOperationException("A native owner identity is bound to a different managed owner.");
        if (pendingOwners.ContainsKey(owner.Identity))
            throw new InvalidOperationException("A native component reservation already owns this GameObject.");

        EnsureReservationCapacity(existingRoots is null);
        Reservation preparation = new(owner, existingRoots);
        pendingOwners.Add(owner.Identity, preparation);
        return preparation;
    }

    // Call immediately after native reserve returns. Capacity was reserved by
    // PrepareReservation, so this records identity without growing storage.
    internal Reservation RegisterPending(long identity, Reservation preparation)
    {
        if (identity <= 0)
            throw new ArgumentOutOfRangeException(nameof(identity));
        ArgumentNullException.ThrowIfNull(preparation);
        if (preparation.State != ReservationState.Prepared)
            throw new InvalidOperationException("The reservation preparation is no longer available.");
        RequirePendingOwner(preparation);
        if (committed.ContainsKey(identity) || pending.ContainsKey(identity))
            throw new InvalidOperationException("A native component identity is already attached.");

        pending.Add(identity, preparation);
        preparation.Identity = identity;
        preparation.State = ReservationState.Pending;
        return preparation;
    }

    // All allocations and ownership checks happen before native publication.
    internal void PrepareCommit(Reservation reservation, Component wrapper)
    {
        RequirePending(reservation);
        RequirePendingOwner(reservation);
        ArgumentNullException.ThrowIfNull(wrapper);
        if (wrapper.Identity != reservation.Identity ||
            !ReferenceEquals(wrapper.AttachedOwner, reservation.Owner) ||
            !ReferenceEquals(wrapper.Context, reservation.Owner.Context))
            throw new InvalidOperationException("The native wrapper does not match its reservation owner.");

        OwnerRoots roots = reservation.PrepareRoots(rootsByOwner);
        reservation.PreparedRoots = roots;
        roots.Components.EnsureCapacity(roots.Components.Count + 1);
        committed.EnsureCapacity(committed.Count + 1);
        reservation.PreparedWrapper = wrapper;
    }

    // Call after PrepareCommit and native commit; transfer is nonallocating.
    internal void CommitPrepared(Reservation reservation)
    {
        RequirePending(reservation);
        RequirePendingOwner(reservation);
        Component wrapper = reservation.PreparedWrapper
            ?? throw new InvalidOperationException("The native wrapper was not prepared for commit.");
        OwnerRoots roots = reservation.PreparedRoots
            ?? throw new InvalidOperationException("The native owner roots were not prepared for commit.");
        RequireRootMapping(reservation.Owner, roots);
        if (committed.ContainsKey(reservation.Identity))
            throw new InvalidOperationException("A native component identity is already attached.");

        if (!pending.Remove(reservation.Identity) ||
            !pendingOwners.Remove(reservation.Owner.Identity))
            throw new InvalidOperationException("The native reservation ownership changed during commit.");

        committed.Add(reservation.Identity, wrapper);
        roots.Components.Add(wrapper);
        reservation.State = ReservationState.Committed;
    }

    // Call before RegisterPending when native reserve or wrapper construction fails.
    internal void CancelPreparation(Reservation preparation)
    {
        ArgumentNullException.ThrowIfNull(preparation);
        if (preparation.State != ReservationState.Prepared)
            throw new InvalidOperationException("Only an unregistered reservation preparation can be cancelled.");
        RequirePendingOwner(preparation);

        if (!pendingOwners.Remove(preparation.Owner.Identity))
            throw new InvalidOperationException("The native owner preparation changed during cancellation.");

        preparation.PreparedWrapper = null;
        preparation.PreparedRoots = null;
        preparation.State = ReservationState.Aborted;
    }

    // Call after native abort succeeds. If managed recovery throws, the
    // reservation remains pending so the caller can retry recovery safely.
    internal void CompleteAbort(Reservation reservation)
    {
        RequirePending(reservation);
        RequirePendingOwner(reservation);
        ValidatePreparedRootRemoval(reservation);

        if (!pending.Remove(reservation.Identity) ||
            !pendingOwners.Remove(reservation.Owner.Identity))
            throw new InvalidOperationException("The native reservation ownership changed during abort.");

        RemovePreparedRootIfUnused(reservation);
        reservation.State = ReservationState.Aborted;
    }

    private void EnsureReservationCapacity(bool needsOwnerRoot)
    {
        committed.EnsureCapacity(committed.Count + 1);
        pending.EnsureCapacity(pending.Count + 1);
        pendingOwners.EnsureCapacity(pendingOwners.Count + 1);
        if (needsOwnerRoot)
            rootsByOwner.EnsureCapacity(rootsByOwner.Count + 1);
    }

    private void RequirePending(Reservation reservation)
    {
        ArgumentNullException.ThrowIfNull(reservation);
        if (!pending.TryGetValue(reservation.Identity, out Reservation? existing) ||
            !ReferenceEquals(existing, reservation))
            throw new InvalidOperationException("The native component reservation is no longer owned by this store.");
    }

    private void RequirePendingOwner(Reservation reservation)
    {
        if (!pendingOwners.TryGetValue(reservation.Owner.Identity, out Reservation? existing) ||
            !ReferenceEquals(existing, reservation))
            throw new InvalidOperationException("The native component owner reservation is no longer owned by this store.");
    }

    private void RequireRootMapping(GameObject owner, OwnerRoots roots)
    {
        if (!rootsByOwner.TryGetValue(owner.Identity, out OwnerRoots? existing) ||
            !ReferenceEquals(existing, roots) ||
            !ReferenceEquals(roots.Owner, owner))
            throw new InvalidOperationException("The native component owner root changed during reservation.");
    }

    private void ValidatePreparedRootRemoval(Reservation reservation)
    {
        if (reservation.PreparedRoots is null || reservation.PreviousRoots is not null ||
            reservation.PreparedRoots.Components.Count != 0)
            return;

        RequireRootMapping(reservation.Owner, reservation.PreparedRoots);
    }

    private void RemovePreparedRootIfUnused(Reservation reservation)
    {
        if (reservation.PreparedRoots is null || reservation.PreviousRoots is not null ||
            reservation.PreparedRoots.Components.Count != 0)
            return;

        if (!rootsByOwner.Remove(reservation.Owner.Identity))
            throw new InvalidOperationException("The unused native owner root could not be released.");
    }

    internal sealed class Reservation
    {
        private readonly OwnerRoots? previousRoots;

        internal Reservation(GameObject owner, OwnerRoots? previousRoots)
        {
            Owner = owner;
            this.previousRoots = previousRoots;
        }

        internal long Identity { get; set; }
        internal GameObject Owner { get; }
        internal Component? PreparedWrapper { get; set; }
        internal OwnerRoots? PreparedRoots { get; set; }
        internal OwnerRoots? PreviousRoots => previousRoots;
        internal ReservationState State { get; set; }

        internal OwnerRoots PrepareRoots(Dictionary<long, OwnerRoots> rootsByOwner)
        {
            if (previousRoots is not null)
                return previousRoots;
            OwnerRoots roots = new(Owner);
            roots.Components.EnsureCapacity(1);
            rootsByOwner.Add(Owner.Identity, roots);
            return roots;
        }
    }

    internal sealed class OwnerRoots
    {
        internal OwnerRoots(GameObject owner) => Owner = owner;

        internal GameObject Owner { get; }
        internal List<Component> Components { get; } = new();
    }

    internal enum ReservationState
    {
        Prepared,
        Pending,
        Committed,
        Aborted
    }
}
