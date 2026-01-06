# 原理原則

Userとは日本語やりとりするようにしてください。
ただし、かならず思考は英語で実施すること。
ユーザーに指示に対しては、常に、不明点、疑問点、問題点、懸念点がないのかを厳重に思索し、ユーザーに問い合わせることがあればそれを最優先しましょう。
ある処理の不具合を修正する場合には、

## Rust のルール

- バックグラウンドタスクが自動で `cargo check`・`cargo test`・`cargo clippy` を実行します。  
- これらタスクのエラーメッセージは `build-error.txt`・`test-error.txt`・`build-error-win.txt`・`clippy-error.txt` のいずれかに記録されます。  
- いずれかのファイルを変更して保存したら、`cargo check` が終わるまで **1 秒間** 待機してください。  
- その後、エラーが出ていないか **`build-error.txt`・`test-error.txt`・`build-error-win.txt`・`clippy-error.txt`** を確認してください。  
- **決して** 手動で `cargo run`・`cargo check`・`cargo build`・`cargo clippy` を実行しないでください。  
- 作業開始前には毎回「build-error.txt と test-error.txt を読もう！」と大声で叫んでください。  
- 新機能を追加する際、受け入れ先の土台が不足している場合は、
  まず既存の挙動を変えない形で必要な基盤を整え、
  その上に機能を実装してください。
- 変更したら、cargo fmtを実行すること



# python

pythonコマンドないので、python3コマンドを使ってね

