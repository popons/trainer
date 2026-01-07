# trainer

スロースクワット用の CLI ツールです。ターミナル ASCII アニメ版（`squat`）と、
ブラウザの Canvas で動く Web 版（`squat-web`）を提供します。

## 使い方

### ターミナル版（ASCII）

```
trainer squat
trainer squat --duration 300 --count 20 --countdown 3
```

### Web 版（Canvas）

```
trainer squat-web
trainer squat-web --duration 150 --count 10 --sets 2 --interval 60 --addr 127.0.0.1:12002
```

ブラウザで以下を開きます。

```
http://127.0.0.1:12002
```

## 操作

### ターミナル版

- `SPACE`: 一時停止 / 再開
- `ESC` / `Ctrl+C`: 終了

### Web 版

- 開始: `ENTER`（PC）/ `TAP`（タッチ端末）
- カウントダウン中の `ENTER` / `TAP`: カウントダウンをスキップして即開始
- 一時停止 / 再開: `SPACE`（PC）/ `TAP`（タッチ端末）
- 停止: `ESC` / `Ctrl+C`

## 仕様

- 1 回の動作は「しゃがむ → 5 秒キープ → 立つ」で構成されます。
- `duration / count` が 5 秒以下の場合はエラーになります。
- Web 版では以下の進捗を表示します。
  - 左側: 移動（DOWN/UP）と HOLD の縦進捗
  - 下部: SET 進捗と TOTAL 進捗の水平バー（右側に % 表示）
  - 休憩中: 右中に REST 進捗バーを表示

## オプション

### `trainer squat`

- `--duration <sec>`: 合計時間（秒, default: 300）
- `--count <n>`: 回数（default: 20）
- `--countdown <sec>`: 開始前カウントダウン（秒, default: 3）

### `trainer squat-web`

- `--duration <sec>`: 1 セットの合計時間（秒, default: 150）
- `--count <n>`: 1 セットの回数（default: 10）
- `--sets <n>` / `--set <n>`: セット数（default: 2）
- `--interval <sec>`: セット間インターバル（秒, default: 60）
- `--addr <host:port>`: サーバ待受（default: 127.0.0.1:12002）
- `--swing-start <f>`: 震え開始時の振幅係数（default: 0.4）
- `--swing-stop <f>`: 震え最大時の振幅係数（default: 3.4）
- `--freq <f>`: 震えの周波数（Hz, default: 10.0）

