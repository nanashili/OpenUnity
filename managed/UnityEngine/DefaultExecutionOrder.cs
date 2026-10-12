namespace UnityEngine;

public class DefaultExecutionOrder : Attribute
{
    public DefaultExecutionOrder(int order) => this.order = order;

    public int order { get; }
}
