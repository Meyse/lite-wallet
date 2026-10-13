import AppKit
import ApplicationServices
import Security

// Developer-only automation for explicitly disposable test wallets. Never log input values.
// Only the wallets below are supported, each on one test network. Mainnet is never listed.
struct FixtureSpec: Equatable { let key: String; let name: String; let network: String }
let fixtureSpecs = [
    FixtureSpec(key: "mijn-app", name: "mijn app", network: "testnet"),
    FixtureSpec(key: "private-testnet", name: "Private testnet", network: "privatetestnet"),
]
let testNetworks: Set<String> = ["testnet", "privatetestnet"]
let service = "com.maxtheyse.verus-express.test-automation"
struct Failure: Error { let message: String }
func require(_ ok: Bool, _ message: String) throws { if !ok { throw Failure(message: message) } }
func keychainCheck(_ status: OSStatus) throws {
    try require(status == errSecSuccess, "Keychain operation failed (OSStatus \(status)). No credential was printed.")
}
struct Metadata: Decodable { let id: String; let network: String; let last_unlocked_at: UInt64? }
struct Binding: Codable, Equatable {
    let accountID: String
    let executable: String
    // The git directory that every checkout of this repository shares. Items saved before
    // this field existed name only their checkout's executable.
    var repository: String? = nil
}
struct StoredCredential { let binding: Binding; let bindingData: Data }
let executableSuffix = "/src-tauri/target/debug/verus_express"
func gitCommonDir(_ checkout: URL) -> String? {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
    process.arguments = ["-C", checkout.path, "rev-parse", "--path-format=absolute", "--git-common-dir"]
    let output = Pipe()
    process.standardOutput = output
    process.standardError = FileHandle.nullDevice
    do { try process.run() } catch { return nil }
    let data = output.fileHandleForReading.readDataToEndOfFile()
    process.waitUntilExit()
    guard process.terminationStatus == 0,
          let path = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines),
          path.hasPrefix("/") else { return nil }
    return URL(fileURLWithPath: path).resolvingSymlinksInPath().path
}
func storedRepository(_ binding: Binding) -> String? {
    if let repository = binding.repository { return repository }
    guard binding.executable.hasSuffix(executableSuffix) else { return nil }
    return gitCommonDir(URL(fileURLWithPath: String(binding.executable.dropLast(executableSuffix.count))))
}
/// A saved credential serves its wallet account from any checkout or branch of the same
/// repository. Each unlock still targets only the running build of the current checkout.
func credentialMatches(_ stored: Binding, _ current: Binding, storedRepository repository: String?) -> Bool {
    guard let currentRepository = current.repository else { return false }
    return stored.accountID == current.accountID && repository == currentRepository
}
func credentialMatches(_ stored: Binding, _ current: Binding) -> Bool {
    credentialMatches(stored, current, storedRepository: storedRepository(stored))
}
func bindingData(_ binding: Binding) throws -> Data {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    return try encoder.encode(binding)
}
struct Fixture {
    let repo: URL
    let spec: FixtureSpec
    let repository: String?
    var executable: URL { repo.appendingPathComponent("src-tauri/target/debug/verus_express").resolvingSymlinksInPath() }
    var metadataURL: URL {
        FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(
            "Library/Application Support/com.maxtheyse.verus-express/wallet_data/\(spec.name)_metadata.json")
    }
    func metadata() throws -> Metadata {
        let data: Data
        do { data = try Data(contentsOf: metadataURL) } catch { throw Failure(message: "The named wallet metadata is unavailable.") }
        let result: Metadata
        do { result = try JSONDecoder().decode(Metadata.self, from: data) } catch { throw Failure(message: "Wallet metadata is invalid or has no explicit network.") }
        try validateMetadata(result, spec)
        return result
    }
    func binding() throws -> Binding {
        guard let repository else { throw Failure(message: "This checkout is not part of a git repository.") }
        return Binding(accountID: try metadata().id, executable: executable.path, repository: repository)
    }
}
func validateMetadata(_ metadata: Metadata, _ spec: FixtureSpec) throws {
    try require(testNetworks.contains(spec.network) && metadata.network == spec.network,
                "Refusing a wallet that is not explicitly on \(spec.network).")
    try require(UUID(uuidString: metadata.id) != nil, "Wallet account ID is invalid.")
}
func loginKeychain() throws -> SecKeychain {
    let path = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Keychains/login.keychain-db").path
    var keychain: SecKeychain?
    try keychainCheck(SecKeychainOpen(path, &keychain))
    guard let result = keychain else { throw Failure(message: "Login Keychain is unavailable.") }
    return result
}
func itemQuery(_ keychain: SecKeychain, _ spec: FixtureSpec) -> [String: Any] {
    [kSecClass as String: kSecClassGenericPassword,
     kSecAttrService as String: service, kSecAttrAccount as String: spec.name,
     kSecMatchSearchList as String: [keychain]]
}
func storedCredential(_ keychain: SecKeychain, _ spec: FixtureSpec) throws -> StoredCredential? {
    var query = itemQuery(keychain, spec)
    query[kSecReturnAttributes as String] = true
    query[kSecMatchLimit as String] = kSecMatchLimitOne
    var result: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &result)
    if status == errSecItemNotFound { return nil }
    try keychainCheck(status)
    guard let attrs = result as? [String: Any], let data = attrs[kSecAttrGeneric as String] as? Data,
          let binding = try? JSONDecoder().decode(Binding.self, from: data) else {
        throw Failure(message: "The dedicated credential has an invalid binding; remove it before setup.")
    }
    return StoredCredential(binding: binding, bindingData: data)
}
func passwordPrompt(_ spec: FixtureSpec) throws -> Data {
    let app = NSApplication.shared
    app.setActivationPolicy(.accessory)
    let alert = NSAlert()
    alert.messageText = "Save test wallet password"
    alert.informativeText = "Enter the existing password for ‘\(spec.name)’ (disposable \(spec.network) wallet). It will be stored in your login Keychain for local testing."
    alert.addButton(withTitle: "Save in Keychain")
    alert.addButton(withTitle: "Cancel")
    let field = NSSecureTextField(frame: NSRect(x: 0, y: 0, width: 360, height: 26))
    field.placeholderString = "Wallet password"
    alert.accessoryView = field
    alert.window.initialFirstResponder = field
    app.activate(ignoringOtherApps: true)
    guard alert.runModal() == .alertFirstButtonReturn else { throw Failure(message: "Setup cancelled; no credential changed.") }
    defer { field.stringValue = "" }
    try require(!field.stringValue.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty, "An empty password was not saved.")
    return Data(field.stringValue.utf8)
}
func save(_ keychain: SecKeychain, _ fixture: Fixture) throws {
    let binding = try fixture.binding()
    if let old = try storedCredential(keychain, fixture.spec) {
        try require(credentialMatches(old.binding, binding),
                    "Credential belongs to a different wallet account or repository. Remove it explicitly before setup.")
    }
    var secret = try passwordPrompt(fixture.spec)
    defer { secret.resetBytes(in: 0..<secret.count) }
    try require(try fixture.binding() == binding, "Wallet changed during entry; nothing saved.")
    let attrs: [String: Any] = [kSecValueData as String: secret,
        kSecAttrGeneric as String: try bindingData(binding),
        kSecAttrLabel as String: "Verus Express — \(fixture.spec.name) — disposable \(fixture.spec.network) automation"]
    if let current = try storedCredential(keychain, fixture.spec) {
        try require(credentialMatches(current.binding, binding), "Credential binding changed during entry; nothing saved.")
        var updateQuery = itemQuery(keychain, fixture.spec)
        updateQuery[kSecAttrGeneric as String] = current.bindingData
        try keychainCheck(SecItemUpdate(updateQuery as CFDictionary, attrs as CFDictionary))
    } else {
        // A single trusted application, never an all-applications ACL.
        var trusted: SecTrustedApplication?
        try keychainCheck(SecTrustedApplicationCreateFromPath(nil, &trusted))
        guard let trusted else { throw Failure(message: "Cannot establish helper identity.") }
        var access: SecAccess?
        try keychainCheck(SecAccessCreate("Disposable testnet wallet automation" as CFString, [trusted] as CFArray, &access))
        guard let access else { throw Failure(message: "Cannot create restricted Keychain access.") }
        var query = attrs
        query[kSecClass as String] = kSecClassGenericPassword
        query[kSecAttrService as String] = service
        query[kSecAttrAccount as String] = fixture.spec.name
        query[kSecUseKeychain as String] = keychain
        query[kSecAttrAccess as String] = access
        try keychainCheck(SecItemAdd(query as CFDictionary, nil))
    }
    print("Saved the dedicated \(fixture.spec.network) credential in login Keychain. Password correctness is not yet verified.")
}
func readSecret(_ keychain: SecKeychain, _ spec: FixtureSpec, _ binding: Binding) throws -> Data {
    guard let stored = try storedCredential(keychain, spec), credentialMatches(stored.binding, binding) else {
        throw Failure(message: "No matching credential. Run setup first.")
    }
    var query = itemQuery(keychain, spec)
    query[kSecAttrGeneric as String] = stored.bindingData
    query[kSecReturnData as String] = true
    query[kSecMatchLimit as String] = kSecMatchLimitOne
    var value: CFTypeRef?
    try keychainCheck(SecItemCopyMatching(query as CFDictionary, &value))
    guard let data = value as? Data, !data.isEmpty else { throw Failure(message: "Stored credential is invalid.") }
    return data
}
func stringAttribute(_ element: AXUIElement, _ name: String) -> String {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success else { return "" }
    return value as? String ?? ""
}
func children(_ element: AXUIElement) -> [AXUIElement] {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &value) == .success else { return [] }
    return value as? [AXUIElement] ?? []
}
func isFixtureWalletSelectorLabel(_ label: String, _ name: String) -> Bool {
    return [
        "Switch wallet, current wallet: \(name)",
        "Wallet wisselen, huidige wallet: \(name)",
    ].contains(label)
}
struct ScreenFacts {
    var names = 0; var secureFields = 0; var inputs = 0; var webAreas = 0; var windows = 0
    var modal = false; var truncated = false; var fieldSettable = false
}
func validateScreen(_ facts: ScreenFacts, _ spec: FixtureSpec) throws {
    try require(!facts.truncated && !facts.modal && facts.windows == 1 && facts.webAreas == 1,
                "Refusing an ambiguous wallet window or an open dialog (windows: \(facts.windows), web areas: \(facts.webAreas), modal: \(facts.modal), truncated: \(facts.truncated)).")
    try require(facts.names == 1 && facts.secureFields == 1 && facts.inputs == 1 && facts.fieldSettable,
                "Select ‘\(spec.name)’ on the normal unlock screen with the password hidden, then retry.")
}
struct Target {
    let app: NSRunningApplication; let field: AXUIElement; let root: AXUIElement
}
func target(_ fixture: Fixture) throws -> Target {
    try require(AXIsProcessTrusted(), "Accessibility is unavailable. Run this helper from an authorized local terminal outside the command sandbox; do not disable macOS protections.")
    let matches = NSWorkspace.shared.runningApplications.filter {
        $0.executableURL?.resolvingSymlinksInPath().path == fixture.executable.path
    }
    try require(matches.count == 1, "Exactly one wallet from this checkout must be running (pnpm tauri dev).")
    let app = matches[0]
    let root = AXUIElementCreateApplication(app.processIdentifier)
    var facts = ScreenFacts(), field: AXUIElement?, count = 0
    func visit(_ element: AXUIElement, depth: Int) {
        count += 1
        if count > 3000 || depth > 40 { facts.truncated = true; return }
        let role = stringAttribute(element, kAXRoleAttribute)
        if role == "AXWindow" { facts.windows += 1 }
        if role == "AXWebArea" { facts.webAreas += 1 }
        if role == "AXSheet" || role == "AXDialog" || stringAttribute(element, kAXSubroleAttribute) == "AXDialog" { facts.modal = true }
        if role == "AXStaticText", stringAttribute(element, kAXValueAttribute) == fixture.spec.name {
            facts.names += 1
        } else if role == "AXPopUpButton" {
            // WebKit may expose the button's visible contents as one accessible label.
            // Match only the exact localized label for the designated fixture wallet.
            let title = stringAttribute(element, kAXTitleAttribute)
            let description = stringAttribute(element, kAXDescriptionAttribute)
            if isFixtureWalletSelectorLabel(title, fixture.spec.name)
                || isFixtureWalletSelectorLabel(description, fixture.spec.name) {
                facts.names += 1
            }
        }
        if role == "AXTextField" || role == "AXTextArea" {
            facts.inputs += 1
            // Never read AXValue on any input, including revealed password fields.
            if stringAttribute(element, kAXSubroleAttribute) == "AXSecureTextField",
               stringAttribute(element, "AXDOMIdentifier") == "unlock-password" {
                facts.secureFields += 1; field = element
                var settable: DarwinBoolean = false
                facts.fieldSettable = AXUIElementIsAttributeSettable(element, kAXValueAttribute as CFString, &settable) == .success && settable.boolValue
            }
        }
        for child in children(element) { visit(child, depth: depth + 1) }
    }
    visit(root, depth: 0)
    try validateScreen(facts, fixture.spec)
    guard let field else { throw Failure(message: "Secure unlock field is unavailable.") }
    return Target(app: app, field: field, root: root)
}
func boolAttribute(_ element: AXUIElement, _ name: String) -> Bool {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success else { return false }
    return value as? Bool ?? false
}
func focusSecureField(_ fixture: Fixture, _ expected: Target) throws {
    // Focus the exact guarded element. WebKit secure AX value setters do not reliably
    // dispatch input events, so entry uses process-targeted keys, never global posting.
    try require(AXUIElementSetAttributeValue(expected.root, kAXFrontmostAttribute as CFString, kCFBooleanTrue) == .success,
                "The guarded wallet application could not be brought forward.")
    try require(AXUIElementSetAttributeValue(expected.field, kAXFocusedAttribute as CFString, kCFBooleanTrue) == .success,
                "The guarded secure field could not be focused.")
    for _ in 0..<20 {
        let fresh = try target(fixture)
        try require(sameTarget(expected, fresh), "Wallet changed while focusing the secure field.")
        if boolAttribute(fresh.root, kAXFrontmostAttribute) && boolAttribute(fresh.field, kAXFocusedAttribute) { return }
        Thread.sleep(forTimeInterval: 0.05)
    }
    throw Failure(message: "The guarded secure field did not receive focus.")
}
func sendKey(_ code: CGKeyCode, flags: CGEventFlags = [], text: String? = nil,
             fixture: Fixture, expected: Target, binding: Binding) throws {
    let fresh = try target(fixture)
    try require(sameTarget(expected, fresh) && (try fixture.binding()) == binding &&
                boolAttribute(fresh.root, kAXFrontmostAttribute) && boolAttribute(fresh.field, kAXFocusedAttribute),
                "Wallet or secure-field focus changed during entry; stopped.")
    guard let source = CGEventSource(stateID: .privateState),
          let down = CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: true),
          let up = CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: false) else {
        throw Failure(message: "Native keyboard events are unavailable.")
    }
    down.flags = flags; up.flags = flags
    if let text {
        let units = Array(text.utf16)
        down.keyboardSetUnicodeString(stringLength: units.count, unicodeString: units)
        up.keyboardSetUnicodeString(stringLength: units.count, unicodeString: units)
    }
    // These events go only to the verified wallet PID, never the global event stream.
    down.postToPid(fresh.app.processIdentifier)
    up.postToPid(fresh.app.processIdentifier)
    Thread.sleep(forTimeInterval: 0.02)
}
func clearSecureField(_ fixture: Fixture, _ expected: Target, _ binding: Binding) throws {
    try sendKey(0, flags: .maskCommand, fixture: fixture, expected: expected, binding: binding)
    try sendKey(51, fixture: fixture, expected: expected, binding: binding)
}
func sameTarget(_ first: Target, _ next: Target) -> Bool {
    first.app.processIdentifier == next.app.processIdentifier && CFEqual(first.field, next.field)
}
func isUnlockLabel(_ label: String) -> Bool {
    // Exact idle labels from src/lib/i18n/locales/{en,nl}.ts; never match loading labels.
    ["Unlock", "Ontgrendel"].contains(label)
}
func nextSubmitDelay(previous: UInt64?, now: TimeInterval) -> TimeInterval {
    // Backend timestamps have one-second resolution. Avoid an indistinguishable success.
    guard let previous, Double(previous) == floor(now) else { return 0 }
    return Double(previous) + 1.05 - now
}
func unlockButton(_ root: AXUIElement) throws -> AXUIElement {
    var buttons = [AXUIElement](), count = 0
    func visit(_ element: AXUIElement, depth: Int) {
        count += 1
        guard count <= 3000, depth <= 40 else { return }
        if stringAttribute(element, kAXRoleAttribute) == "AXButton" {
            let title = stringAttribute(element, kAXTitleAttribute)
            let description = stringAttribute(element, kAXDescriptionAttribute)
            if [title, description].contains(where: isUnlockLabel) { buttons.append(element) }
        }
        for child in children(element) { visit(child, depth: depth + 1) }
    }
    visit(root, depth: 0)
    try require(count <= 3000 && buttons.count == 1, "The normal unlock button is unavailable or ambiguous; no submit performed.")
    return buttons[0]
}
func fill(_ keychain: SecKeychain, _ fixture: Fixture, submit: Bool) throws {
    let binding = try fixture.binding()
    let initial = try target(fixture)
    var secret = try readSecret(keychain, fixture.spec, binding)
    defer { secret.resetBytes(in: 0..<secret.count) }
    // Keychain permission dialogs may have appeared. Revalidate after retrieval.
    try require(try fixture.binding() == binding, "Wallet changed during credential retrieval.")
    let checked = try target(fixture)
    try require(sameTarget(initial, checked), "Wallet target changed during credential retrieval.")
    guard let password = String(data: secret, encoding: .utf8) else { throw Failure(message: "Stored credential is not valid UTF-8.") }
    do {
        try focusSecureField(fixture, checked)
        try require(try fixture.binding() == binding, "Wallet changed while focusing the secure field.")
        try clearSecureField(fixture, checked, binding)
        for scalar in password.unicodeScalars {
            try sendKey(0, text: String(scalar), fixture: fixture, expected: checked, binding: binding)
        }
        var accepted = false
        for _ in 0..<20 {
            let updated = try target(fixture)
            try require(sameTarget(checked, updated), "Wallet changed during entry.")
            if boolAttribute(try unlockButton(updated.root), kAXEnabledAttribute) { accepted = true; break }
            Thread.sleep(forTimeInterval: 0.05)
        }
        try require(accepted, "The app did not accept the secure field change; no submit performed.")
        if !submit { print("Filled the guarded secure field and verified Unlock is enabled. Wallet is not yet unlocked."); return }
        let fresh = try target(fixture)
        try require(sameTarget(checked, fresh) && (try fixture.binding()) == binding, "Wallet changed before submit.")
        let previous = try fixture.metadata().last_unlocked_at
        let delay = nextSubmitDelay(previous: previous, now: Date().timeIntervalSince1970)
        if delay > 0 { Thread.sleep(forTimeInterval: delay) }
        let submitTarget = try target(fixture)
        try require(sameTarget(fresh, submitTarget) && (try fixture.binding()) == binding, "Wallet changed before submit.")
        let button = try unlockButton(submitTarget.root)
        try require(AXUIElementPerformAction(button, kAXPressAction as CFString) == .success, "Unlock button could not be pressed.")
        for _ in 0..<100 {
            Thread.sleep(forTimeInterval: 0.2)
            let current = try fixture.metadata()
            try require(current.id == binding.accountID, "Wallet account changed while unlocking.")
            if let stamp = current.last_unlocked_at, stamp != previous {
                print("Unlock confirmed by the \(fixture.spec.network) wallet's updated success timestamp.")
                return
            }
        }
        throw Failure(message: "Unlock was not confirmed within 20 seconds. Check the wallet UI; the password may be incorrect.")
    } catch {
        // Clear only the same secure field if it still belongs to the guarded unlock screen.
        if let current = try? target(fixture), sameTarget(checked, current) {
            if (try? focusSecureField(fixture, current)) != nil {
                try? clearSecureField(fixture, current, binding)
            }
        }
        throw error
    }
}
func selfTest() throws {
    let id = "11111111-1111-4111-8111-111111111111"
    let testnet = fixtureSpecs[0], privateTestnet = fixtureSpecs[1]
    try validateMetadata(Metadata(id: id, network: "testnet", last_unlocked_at: nil), testnet)
    try validateMetadata(Metadata(id: id, network: "privatetestnet", last_unlocked_at: nil), privateTestnet)
    var rejected = 0
    for (metadata, spec) in [
        (Metadata(id: id, network: "mainnet", last_unlocked_at: nil), testnet),
        (Metadata(id: "", network: "testnet", last_unlocked_at: nil), testnet),
        // Each wallet stays on its own test network.
        (Metadata(id: id, network: "privatetestnet", last_unlocked_at: nil), testnet),
        (Metadata(id: id, network: "testnet", last_unlocked_at: nil), privateTestnet),
        (Metadata(id: id, network: "mainnet", last_unlocked_at: nil), privateTestnet),
        // A wallet listed on a network outside the test networks is refused.
        (Metadata(id: id, network: "mainnet", last_unlocked_at: nil),
         FixtureSpec(key: "mainnet", name: "mainnet", network: "mainnet")),
    ] {
        do { try validateMetadata(metadata, spec) } catch { rejected += 1 }
    }
    try require(fixtureSpecs.allSatisfy { testNetworks.contains($0.network) }, "Only test networks may be listed.")
    let valid = ScreenFacts(names: 1, secureFields: 1, inputs: 1, webAreas: 1, windows: 1, modal: false, truncated: false, fieldSettable: true)
    try validateScreen(valid, testnet)
    for variant in 0..<9 {
        var facts = valid
        switch variant { case 0: facts.names = 0; case 1: facts.names = 2; case 2: facts.secureFields = 0
        case 3: facts.inputs = 2; case 4: facts.modal = true; case 5: facts.truncated = true
        case 6: facts.fieldSettable = false; case 7: facts.windows = 2; default: facts.webAreas = 2 }
        do { try validateScreen(facts, testnet) } catch { rejected += 1 }
    }
    try require(rejected == 15, "Guard regression test failed.")
    for spec in fixtureSpecs {
        try require(isFixtureWalletSelectorLabel("Switch wallet, current wallet: \(spec.name)", spec.name),
                    "English wallet selector label must be accepted.")
        try require(isFixtureWalletSelectorLabel("Wallet wisselen, huidige wallet: \(spec.name)", spec.name),
                    "Dutch wallet selector label must be accepted.")
    }
    try require(!isFixtureWalletSelectorLabel("Switch wallet, current wallet: other", testnet.name),
                "Another wallet selector must be rejected.")
    try require(!isFixtureWalletSelectorLabel("Switch wallet, current wallet: \(privateTestnet.name)", testnet.name),
                "The other test wallet's selector must be rejected.")
    // One saved credential serves its account from any checkout of the same repository.
    let repoA = "/test-only/repo/.git"
    let here = Binding(accountID: id, executable: "/test-only/repo/src-tauri/target/debug/verus_express", repository: repoA)
    let worktree = Binding(accountID: id, executable: "/test-only/repo/.claude/worktrees/x/src-tauri/target/debug/verus_express", repository: repoA)
    try require(credentialMatches(here, worktree, storedRepository: repoA), "Another checkout of the same repository must match.")
    try require(!credentialMatches(here, Binding(accountID: id, executable: worktree.executable, repository: "/test-only/other/.git"), storedRepository: repoA),
                "Another repository must be rejected.")
    try require(!credentialMatches(here, Binding(accountID: "22222222-2222-4222-8222-222222222222", executable: worktree.executable, repository: repoA), storedRepository: repoA),
                "Another wallet account must be rejected.")
    try require(!credentialMatches(here, Binding(accountID: id, executable: worktree.executable), storedRepository: repoA),
                "A checkout without a repository must be rejected.")
    try require(!credentialMatches(here, worktree, storedRepository: nil), "An unresolvable saved repository must be rejected.")
    let legacy = try JSONDecoder().decode(Binding.self, from: Data(#"{"accountID":"\#(id)","executable":"/test-only/verus_express"}"#.utf8))
    try require(legacy.repository == nil, "Items saved before repository binding must still decode.")
    let dummy = "test-only 🔑 é漢字 quotes'\"$`"
    try require(String(data: Data(dummy.utf8), encoding: .utf8) == dummy, "UTF-8 round-trip failed.")
    try require(dummy.unicodeScalars.map(String.init).joined() == dummy, "Unicode event chunks must preserve the full password.")
    let binding = Binding(accountID: id, executable: "/test-only/verus_express")
    try require(try bindingData(binding) == bindingData(binding), "Binding encoding must be deterministic.")
    try require(try JSONDecoder().decode(Binding.self, from: bindingData(binding)) == binding, "Binding round-trip failed.")
    try require(isUnlockLabel("Unlock") && isUnlockLabel("Ontgrendel"), "Configured unlock labels must match.")
    try require(!isUnlockLabel("Unlocking…") && !isUnlockLabel("Ontgrendelen…") && !isUnlockLabel("Ontgrendelen"), "Loading and incorrect labels must not match.")
    try require(nextSubmitDelay(previous: 100, now: 100.5) > 0.5 && nextSubmitDelay(previous: 100, now: 101) == 0 && nextSubmitDelay(previous: nil, now: 100) == 0, "Rapid re-unlock timing guard failed.")
    print("Passed metadata guards for \(fixtureSpecs.count) test wallets, 9 unsafe screen cases, repository bindings, Unicode, locale labels, and rapid re-unlock timing. No Keychain or UI access used.")
}
let args = Array(CommandLine.arguments.dropFirst())
do {
    try require(args.count == 3, "Usage: scripts/test-wallet-keychain.sh setup|status|preflight|fill|unlock|remove|self-test [mijn-app|private-testnet]")
    let command = args[0]
    guard let spec = fixtureSpecs.first(where: { $0.key == args[2] }) else {
        throw Failure(message: "Unknown test wallet. Use mijn-app or private-testnet.")
    }
    let repo = URL(fileURLWithPath: args[1]).resolvingSymlinksInPath()
    let fixture = Fixture(repo: repo, spec: spec, repository: gitCommonDir(repo))
    switch command {
    case "self-test": try selfTest()
    case "preflight": _ = try fixture.binding(); _ = try target(fixture); print("Ready: named \(spec.network) wallet, exact development executable, and guarded secure field verified. No Keychain access.")
    case "setup": try save(loginKeychain(), fixture)
    case "status":
        let binding = try fixture.binding()
        if let stored = try storedCredential(loginKeychain(), spec) {
            try require(credentialMatches(stored.binding, binding),
                        "Stored credential belongs to a different wallet account or repository.")
            print("Dedicated credential is present and matches this \(spec.network) account in this repository. Password was not retrieved.")
        } else { print("No dedicated credential saved. Run setup.") }
    case "fill", "unlock": try fill(loginKeychain(), fixture, submit: command == "unlock")
    case "remove":
        let status = SecItemDelete(itemQuery(try loginKeychain(), spec) as CFDictionary)
        if status != errSecItemNotFound { try keychainCheck(status) }
        print("Dedicated ‘\(spec.name)’ testing credential removed (or already absent). Wallet unchanged.")
    default: throw Failure(message: "Unknown command. Use setup, status, preflight, fill, unlock, remove, or self-test.")
    }
} catch let error as Failure {
    fputs(error.message + "\n", stderr); exit(1)
} catch {
    fputs("Helper failed. No credential was printed.\n", stderr); exit(1)
}
