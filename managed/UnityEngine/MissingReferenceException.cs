namespace UnityEngine;

public class MissingReferenceException : SystemException
{
    public MissingReferenceException() { }

    public MissingReferenceException(string? message) : base(message) { }

    public MissingReferenceException(string? message, Exception? innerException)
        : base(message, innerException) { }
}
