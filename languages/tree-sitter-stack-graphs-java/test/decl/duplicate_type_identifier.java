// TODO: this triggers a Duplicate variable error
// Caused by: Duplicate variable [syntax node type_identifier (3, 5)].value set at (553, 14) and (553, 14)
// Unclear to me why it lists that line/row twice?
public class RepairScanner implements MessageStateProcessor {
  private Map<TopicPartition, OffsetAndMetadata> fetchRelayOffsets() {
    Map<TopicPartition, OffsetAndMetadata> offsets = Collections.emptyMap();
  }
}
