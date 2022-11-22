N_SERVERS=5
N_CLIENTS_PER_SERVER=10

rm -rf cafeteras/logs
rm -rf servidores/logs
rm -rf servidores/databases

cargo build --bin server
cargo build --bin coffee_maker

echo "Builds hechas"

declare -a pid_servers
for i in $(seq 1 $N_SERVERS)
do
    ../../target/debug/server servidores/cfg_$i.json > /dev/null &
    pid_servers[i - 1]=$!
done

echo "Servidores iniciados: ${pid_servers[*]}"
sleep 2

declare -a pid_clients
for i in $(seq 1 $N_SERVERS)
do
    for j in $(seq 1 $N_CLIENTS_PER_SERVER)
    do
        ../../target/debug/coffee_maker cafeteras/cfg_$i.json > /dev/null &
        pid_clients[i*N_CLIENTS_PER_SERVER+j - 1]=$!
    done
done

echo "Cafeteras iniciadas: ${pid_clients[*]}"

for pid in ${pid_clients[*]}
do
    wait $pid
    echo "Cafetera $pid terminó"
done

echo "Parando servidores"
# Sleep de un poco más que "add_points_interval_ms" en la config de los servers para que terminen
# de procesar los puntos
sleep 3
for pid in ${pid_servers[*]}
do
    kill -INT $pid
    echo "Servidor $pid parado"
done
