rm -rf cafeteras/logs
rm -rf servidores/logs

cargo build --bin server
cargo build --bin coffee_maker

cargo run --bin server servidores/cfg_1.json > /dev/null &
pids1=$!
cargo run --bin server servidores/cfg_2.json > /dev/null &
pids2=$!
cargo run --bin server servidores/cfg_3.json > /dev/null &
pids3=$!

echo "Servidores iniciados: $pids1 $pids2 $pids3"

sleep 1

cargo run --bin coffee_maker cafeteras/cfg_1.json &
pid1=$!
cargo run --bin coffee_maker cafeteras/cfg_2.json &
pid2=$!
cargo run --bin coffee_maker cafeteras/cfg_3.json &
pid3=$!


wait $pid1
echo "Cafetera 1 terminó"
wait $pid2
echo "Cafetera 2 terminó"
wait $pid3
echo "Cafetera 3 terminó"

echo "Parando servidores"
# Sleep de un poco más que "add_points_interval_ms" en la config de los servers para que terminen
# de procesar los puntos
sleep 1 
kill -INT $pids1
kill -INT $pids2
kill -INT $pids3
