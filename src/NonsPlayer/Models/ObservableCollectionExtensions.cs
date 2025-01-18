using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Collections.Specialized;
using System.ComponentModel;
using System.Linq;
using System.Reflection;

namespace NonsPlayer.Models
{
    public static class ObservableCollectionExtensions
    {
        private static BindingFlags ProtectedMember = BindingFlags.Instance | BindingFlags.InvokeMethod | BindingFlags.NonPublic;
        private static BindingFlags ProtectedProperty = BindingFlags.Instance | BindingFlags.FlattenHierarchy | BindingFlags.NonPublic;

        /// <summary>
        /// Insert a collection without triggering OnCollectionChanged event 
        /// </summary>
        private static void InsertWithoutNotify<T>(this ObservableCollection<T> collection, IEnumerable<T> items, int index = -1)
        {
            if (collection == null || items == null || !items.Any()) return;
            Type type = collection.GetType();

            type.InvokeMember("CheckReentrancy", ProtectedMember, null, collection, null);

            PropertyInfo itemsProp = type.BaseType.GetProperty("Items", ProtectedProperty);
            IList<T> protectedItems = itemsProp.GetValue(collection) as IList<T>;

            // Behave the same as Add if no index is being passed
            int start = index > -1 ? index : protectedItems.Count();
            int end = items.Count();
            for (int i = 0; i < end; i++)
            {
                protectedItems.Insert(start + i, items.ElementAt(i));
            }

            type.InvokeMember("OnPropertyChanged", ProtectedMember, null,
              collection, new object[] { new PropertyChangedEventArgs("Count") });

            type.InvokeMember("OnPropertyChanged", ProtectedMember, null,
              collection, new object[] { new PropertyChangedEventArgs("Item[]") });
        }

        public static void AddRange<T>(this ObservableCollection<T> collection, IEnumerable<T> items)
        {
            if (collection == null || items == null || !items.Any()) return;

            Type type = collection.GetType();

            InsertWithoutNotify(collection, items);

            type.InvokeMember("OnCollectionChanged", ProtectedMember, null,
              collection, new object[] {
                  // Notify that we've added new items into the collection
                  new NotifyCollectionChangedEventArgs(NotifyCollectionChangedAction.Reset)
              });
        }

        public static void InsertRange<T>(this ObservableCollection<T> collection, int index, IEnumerable<T> items)
        {
            if (collection == null || items == null || !items.Any()) return;

            Type type = collection.GetType();

            InsertWithoutNotify(collection, items, index);

            type.InvokeMember("OnCollectionChanged", ProtectedMember, null,
              collection, new object[] {
                  // Notify that we've added new items into the collection
                  new NotifyCollectionChangedEventArgs(NotifyCollectionChangedAction.Reset)
              });
        }

        public static void ReplaceCollection<T>(this ObservableCollection<T> collection, IEnumerable<T> items)
        {
            if (collection == null || items == null || !items.Any()) return;

            Type type = collection.GetType();

            // Clear the underlaying list items without triggering a change
            PropertyInfo itemsProp = type.BaseType.GetProperty("Items", ProtectedProperty);
            IList<T> protectedItems = itemsProp.GetValue(collection) as IList<T>;
            protectedItems.Clear();

            // Perform the actual update
            InsertWithoutNotify(collection, items);

            type.InvokeMember("OnCollectionChanged", ProtectedMember, null,
                collection, new object[] {
                    // Notify that we have replaced the entire collection
                    new NotifyCollectionChangedEventArgs(NotifyCollectionChangedAction.Reset)
                });
        }
    }
}
