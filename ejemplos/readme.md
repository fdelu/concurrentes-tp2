# Ejemplos

Esta carpeta contiene ejemplos sobre el uso del servidor y la cafetera. Cada carpeta tiene un README con la forma de ejecución y el resultado esperado.

## Solución de problemas

En MacOS, a veces falla bindear las ips locales. Para arreglarlo, ejecutar
`sudo ifconfig lo0 alias (ip) up` con `(ip)` de `127.0.0.1` a `127.0.0.5`.
