using System.Collections;

namespace UnityEngine;

internal sealed class TransformChildEnumerator : IEnumerator, IDisposable
{
    private enum EnumeratorState
    {
        Active,
        Exhausted,
        Disposed
    }

    private Transform? parent;
    private Transform? current;
    private int nextIndex;
    private EnumeratorState state;

    internal TransformChildEnumerator(Transform parent)
    {
        this.parent = parent;
    }

    object? IEnumerator.Current => current ?? throw new InvalidOperationException(
        "The Transform child enumerator is not positioned on a child.");

    public bool MoveNext()
    {
        if (state != EnumeratorState.Active)
            return false;

        Transform activeParent = parent ?? throw new InvalidOperationException(
            "The Transform child enumerator lost its parent before disposal.");
        if (nextIndex >= activeParent.childCount)
        {
            current = null;
            state = EnumeratorState.Exhausted;
            return false;
        }

        current = activeParent.GetChild(nextIndex);
        nextIndex++;
        return true;
    }

    public void Reset()
    {
        if (state == EnumeratorState.Disposed)
            throw new ObjectDisposedException(nameof(TransformChildEnumerator));

        nextIndex = 0;
        current = null;
        state = EnumeratorState.Active;
    }

    public void Dispose()
    {
        if (state == EnumeratorState.Disposed)
            return;

        current = null;
        parent = null;
        state = EnumeratorState.Disposed;
    }
}
