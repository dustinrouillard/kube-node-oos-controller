# Kubernetes OOS Controller

This will watch for nodes to become unreachable, by the taint `node.kubernetes.io/unreachable`, and then apply the `node.kubernetes.io/out-of-service` taint with value set to `nodeshutdown` with an effect of `NoExecute`.

Which will make kubernetes do the following:

- Remove any volume attachments from the lost node
- Delete the old pods on the lost node forcefully

This allows any statefulsets or pods with a mounted pvc to be recreated.