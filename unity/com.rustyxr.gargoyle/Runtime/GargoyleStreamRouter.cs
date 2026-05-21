using UnityEngine;

namespace RustyXr.Gargoyle
{
    [DisallowMultipleComponent]
    [DefaultExecutionOrder(-17)]
    public sealed class GargoyleStreamRouter : MonoBehaviour
    {
        [SerializeField] GargoyleClient client;
        [SerializeField] bool autoDiscoverReceivers = true;
        [SerializeField] GargoyleStreamReceiver[] receivers = { };

        int _routedEvents;
        int _unhandledEvents;
        bool _subscribed;

        public int RoutedEvents => _routedEvents;
        public int UnhandledEvents => _unhandledEvents;

        void OnEnable()
        {
            ResolveReferences(false);
            Subscribe();
        }

        void OnDisable()
        {
            Unsubscribe();
        }

        public void ConfigureReferences(GargoyleClient gargoyleClient, params GargoyleStreamReceiver[] streamReceivers)
        {
            Unsubscribe();
            client = gargoyleClient;
            receivers = streamReceivers ?? System.Array.Empty<GargoyleStreamReceiver>();
            Subscribe();
        }

        public bool RouteStreamEvent(GargoyleStreamEvent streamEvent, string rawJson)
        {
            ResolveReferences(false);
            var handled = false;
            if (receivers != null)
            {
                for (var i = 0; i < receivers.Length; i++)
                {
                    var receiver = receivers[i];
                    if (receiver != null && receiver.Receive(streamEvent, rawJson))
                    {
                        handled = true;
                    }
                }
            }

            if (handled)
            {
                _routedEvents++;
            }
            else
            {
                _unhandledEvents++;
            }

            return handled;
        }

        void Subscribe()
        {
            if (_subscribed || client == null)
            {
                return;
            }

            client.StreamEventReceived += HandleStreamEventReceived;
            _subscribed = true;
        }

        void Unsubscribe()
        {
            if (!_subscribed || client == null)
            {
                _subscribed = false;
                return;
            }

            client.StreamEventReceived -= HandleStreamEventReceived;
            _subscribed = false;
        }

        void HandleStreamEventReceived(GargoyleStreamEvent streamEvent, string rawJson)
        {
            RouteStreamEvent(streamEvent, rawJson);
        }

        void ResolveReferences(bool force)
        {
            if ((client == null || force) && TryGetComponent(out GargoyleClient localClient))
            {
                client = localClient;
            }

            if (client == null || force)
            {
                client = FindAnyObjectByType<GargoyleClient>();
            }

            if (autoDiscoverReceivers && (receivers == null || receivers.Length == 0 || force))
            {
                receivers = GetComponentsInChildren<GargoyleStreamReceiver>(true);
            }
        }
    }
}
