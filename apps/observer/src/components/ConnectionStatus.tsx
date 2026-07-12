function ConnectionStatus() {
  return (
    <div>
      <span>Protocol: v1</span>
      {" | "}
      <span>Locale: en-US</span>
      {" | "}
      <span style={{ color: "orange" }}>No simulation attached</span>
    </div>
  );
}

export default ConnectionStatus;
