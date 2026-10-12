using System.Runtime.ExceptionServices;

namespace UnityEngine;

internal interface INativeComponentReservationHost
{
    NativeComponentAttachments.Reservation PrepareReservation(GameObject owner);
    NativeComponentAttachments.Reservation RegisterPending(long identity,
        NativeComponentAttachments.Reservation preparation);
    void CancelPreparation(NativeComponentAttachments.Reservation preparation);
    void PrepareCommit(NativeComponentAttachments.Reservation reservation, Component wrapper);
    void CommitPrepared(NativeComponentAttachments.Reservation reservation);
    void CompleteAbort(NativeComponentAttachments.Reservation reservation);
    void RestoreOwner(NativeComponentAttachments.Reservation reservation);
    void RetainRecovery(NativeComponentReservation reservation);
    void ReleaseRecovery(NativeComponentReservation reservation);
}

// Factory bindings use this marker only when native returned a known negative
// status. A thrown call failure remains distinguishable and poisons recovery.
internal sealed class NativeComponentKnownRejectionException : Exception
{
    internal NativeComponentKnownRejectionException(Exception statusFailure)
        : base("Native component reservation was rejected by a returned status.", statusFailure) =>
        StatusFailure = statusFailure;

    internal Exception StatusFailure { get; }
}

// Binding setup and capability checks throw this marker before entering the
// reserve P/Invoke. No native identity can exist in this state.
internal sealed class NativeComponentReservationNotStartedException : Exception
{
    internal NativeComponentReservationNotStartedException(Exception original)
        : base("Native component reservation did not start.", original) => Original = original;

    internal Exception Original { get; }
}

internal sealed class NativeComponentReservation
{
    private enum NativeState { Prepared, ReserveInFlight, Owned, Aborted, Committed, Unknown }
    private enum RecoveryStep { Pending, Complete }

    private sealed class TransactionState
    {
        internal NativeState Native = NativeState.Prepared;
        internal RecoveryStep Owner = RecoveryStep.Pending;
        internal RecoveryStep Managed = RecoveryStep.Pending;
        internal bool PendingRegistered;
    }

    private readonly RuntimeContext context;
    private readonly INativeComponentReservationHost host;
    private readonly INativeComponentFactoryBinding factory;
    private NativeComponentAttachments.Reservation? attachment;
    private readonly GameObject owner;
    private readonly TransactionState state = new();
    private long identity;
    private Component? wrapper;

    private NativeComponentReservation(RuntimeContext context, INativeComponentReservationHost host,
        INativeComponentFactoryBinding factory, GameObject owner)
    {
        this.context = context;
        this.host = host;
        this.factory = factory;
        this.owner = owner;
    }

    internal Component Wrapper => wrapper
        ?? throw new InvalidOperationException("The native component wrapper is not prepared.");

    internal NativeComponentAttachments.Reservation? PreparedAttachment => attachment;

    internal static NativeComponentReservation Begin(RuntimeContext context,
        INativeComponentReservationHost host, NativeComponentBinding binding, GameObject owner)
    {
        INativeComponentFactoryBinding factory = binding.Factory
            ?? throw new NotSupportedException($"OpenUnity cannot attach {binding.DeclaredType.FullName}.");
        NativeComponentReservation transaction = new(context, host, factory, owner);
        transaction.attachment = host.PrepareReservation(owner);
        try
        {
            transaction.state.Native = NativeState.ReserveInFlight;
            long reserved = factory.Reserve(context, owner);
            transaction.state.Native = NativeState.Prepared;
            transaction.identity = reserved;
            if (reserved <= 0)
                throw RuntimeContext.Failure("native component reservation", reserved);

            transaction.state.Native = NativeState.Owned;
            transaction.attachment = host.RegisterPending(reserved, transaction.Attachment);
            transaction.state.PendingRegistered = true;
            _ = checked((int)reserved);
            return transaction;
        }
        catch (Exception failure)
        {
            if (failure is NativeComponentKnownRejectionException or NativeComponentReservationNotStartedException)
                transaction.state.Native = NativeState.Prepared;
            else if (transaction.state.Native == NativeState.ReserveInFlight)
                transaction.state.Native = NativeState.Unknown;
            Exception primary = failure switch
            {
                NativeComponentKnownRejectionException knownFailure => knownFailure.StatusFailure,
                NativeComponentReservationNotStartedException notStarted => notStarted.Original,
                _ => failure,
            };
            transaction.Recover(primary);
            if (failure is NativeComponentKnownRejectionException rejected)
                ExceptionDispatchInfo.Capture(rejected.StatusFailure).Throw();
            if (failure is NativeComponentReservationNotStartedException skipped)
                ExceptionDispatchInfo.Capture(skipped.Original).Throw();
            throw;
        }
    }

    internal void Prepare()
    {
        wrapper = factory.CreateAttached(new NativeComponentIdentity(context, identity, owner));
        if (wrapper.Context != context || wrapper.Identity != identity ||
            !ReferenceEquals(wrapper.AttachedOwner, owner))
            throw new InvalidOperationException("OpenUnity returned an inconsistent attached component.");
        if (wrapper.GetType() != factory.DeclaredType)
            throw new InvalidOperationException(
                "OpenUnity returned a component wrapper with an unexpected runtime type.");

        host.PrepareCommit(Attachment, wrapper);
    }

    internal void Commit()
    {
        if (wrapper is null)
            throw new InvalidOperationException("A native component must be prepared before commit.");
        if (state.Native is NativeState.Committed or NativeState.Aborted)
            throw new InvalidOperationException("The native component reservation is already finalized.");

        factory.Commit(context, identity);
        state.Native = NativeState.Committed;
        host.CommitPrepared(Attachment);
        identity = 0;
        wrapper = null;
    }

    internal void Recover(Exception primary)
    {
        ArgumentNullException.ThrowIfNull(primary);
        if (state.Native == NativeState.Committed)
        {
            AggregateException failure = new("Managed publication failed after native component commit.",
                primary, new InvalidOperationException("A committed native component cannot be aborted."));
            host.RetainRecovery(this);
            context.RejectFurtherUse(failure);
            throw failure;
        }

        List<Exception> failures = new();
        TryRecovery(RestorePriorOwner, failures);
        TryRecovery(AbortNativeIfOwned, failures);
        if (state.Native == NativeState.Unknown)
            failures.Add(new InvalidOperationException(
                "The native reservation call failed without returning an identity; ownership cannot be proven."));
        else if (state.Native == NativeState.Aborted && state.PendingRegistered)
            TryRecovery(CompleteManagedCleanup, failures);
        else if (state.Native == NativeState.Aborted)
            TryRecovery(CancelManagedPreparation, failures);
        else if (state.PendingRegistered)
            failures.Add(new InvalidOperationException("Native abort did not complete; managed reservation retained."));
        else
            TryRecovery(CancelManagedPreparation, failures);

        if (failures.Count == 0)
        {
            host.ReleaseRecovery(this);
            return;
        }
        failures.Insert(0, primary);
        AggregateException aggregate = new("Native component attachment and reservation recovery failed.", failures);
        host.RetainRecovery(this);
        context.RejectFurtherUse(aggregate);
        throw aggregate;
    }

    private void RestorePriorOwner()
    {
        if (state.Owner == RecoveryStep.Complete)
            return;
        host.RestoreOwner(Attachment);
        state.Owner = RecoveryStep.Complete;
    }

    private void AbortNativeIfOwned()
    {
        if (state.Native is not NativeState.Owned)
            return;
        factory.Abort(context, identity);
        state.Native = NativeState.Aborted;
        identity = 0;
    }

    private void CompleteManagedCleanup()
    {
        if (state.Managed == RecoveryStep.Complete)
            return;
        host.CompleteAbort(Attachment);
        state.Managed = RecoveryStep.Complete;
    }

    private void CancelManagedPreparation()
    {
        if (state.PendingRegistered || state.Managed == RecoveryStep.Complete)
            return;
        host.CancelPreparation(Attachment);
        state.Managed = RecoveryStep.Complete;
    }

    private NativeComponentAttachments.Reservation Attachment => attachment
        ?? throw new InvalidOperationException("The native component reservation has no managed preparation.");

    private static void TryRecovery(Action operation, List<Exception> failures)
    {
        try
        {
            operation();
        }
        catch (Exception failure)
        {
            failures.Add(failure);
        }
    }
}
