namespace UnityEngine;

public class UnityException : SystemException
{
    public UnityException() { }

    public UnityException(string? message) : base(message) { }

    public UnityException(string? message, Exception? innerException)
        : base(message, innerException) { }
}
