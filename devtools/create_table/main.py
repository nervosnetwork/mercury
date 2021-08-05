import sys
import psycopg2


def connect_db():
	try:
		args = sys.argv
		print(args)
		db = psycopg2.connect(database="mercury", user=str(
			args[1]), password=str(args[2]), host=str(args[3]), port=str(args[4]))
		print("Opened database successfully")

	except Exception as e:
		print(e.args[0])
		return

	return db


def extract_sql(sql):
	f = open("./devtools/create_table/create_table.sql", 'r', True)
	line = ""

	while True:
		ch = f.read(1)

		if ch == '':
			break

		if ch == '\n':
			continue

		line = line + str(ch)

		if ch == ';':
			sql.append(line)
			line = ""

	f.close()
	


if __name__ == "__main__":
	sqls = []
	db = connect_db()
	cursor = db.cursor()
	extract_sql(sqls)

	for cmd in sqls:
		cursor.execute(cmd)

	db.commit()
	db.close()