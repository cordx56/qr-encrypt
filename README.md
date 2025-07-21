# qr-encrypt

[日本語](README.ja.md)

This application is a Rust-powered WASM system that enables users to easily utilize public key cryptography with only frontend code.

[Demo page available here](https://qrenc.cordx.cx)

## How to Use

Here's how to use the application:

### General

When you first access the application, your personal keys will be automatically generated.
This may take a moment, so please wait.

### For Recipients (Receiving Encrypted Messages)

Share your public key with the sender in advance.
You can use either the QR code or copy the text directly.

### For Senders (Sending Encrypted Messages)

1. Obtain the recipient's public key in advance and save it with a recognizable name.
2. Click the "Send Message" button to encrypt using the recipient's public key.
3. Send the message by sharing the displayed QR code or copying the text.

## Security

This system is built using HTML, CSS, JavaScript, and WebAssembly.
Since no data is stored on the server, there's minimal risk of your generated private key being exposed externally.
